param(
    [int]$TotalGames = 100000,
    [int]$BudgetMs = 20,
    [int]$Seed = 42,
    [string]$RunTag = $(Get-Date -Format 'yyyyMMdd-HHmmss'),
    [switch]$BuildCurrent,
    [switch]$UsePoolTrain,
    [int]$PoolGames = 2000,
    [int]$PoolBudgetMs = 200,
    [switch]$Benchmark,
    [int]$BenchmarkGames = 50,
    [int]$BenchmarkBudgetMs = 10000,
    [int]$BenchmarkSeed = 4242
)

$ErrorActionPreference = 'Stop'

$scriptRoot = $PSScriptRoot
$botRoot = Split-Path $scriptRoot -Parent
$tool = Join-Path $scriptRoot 'testing_tool.py'
$zooExe = Join-Path $botRoot 'target\release\zoo_bot.exe'
$mushroomExe = Join-Path $botRoot 'target\release\mushroom.exe'
$currentExe = Join-Path $botRoot 'target\release\mushroom-bot.exe'
$botPool = Join-Path $botRoot 'bot_pool'
$legacyExe1 = Join-Path $botRoot 'mushroom_1.exe'
$legacyExe2 = Join-Path $botRoot 'main.exe'
$logsRoot = Join-Path $botRoot 'data\logs\chaos'
$runRoot = Join-Path $logsRoot $RunTag
$csvRoot = Join-Path $runRoot 'csv'
$trainingCsv = Join-Path $runRoot 'training.csv'
$weightsCurrent = Join-Path $botRoot 'data\current_weights.txt'
$weightsBase = Join-Path $botRoot 'balanced.txt'
$geometry = Join-Path $botRoot 'data\geometry.bin'
$dataBin = Join-Path $botRoot 'data\data.bin'
$mquality = Join-Path $botRoot 'data\mquality.bin'

if ($TotalGames -le 0) {
    throw "TotalGames must be positive"
}

New-Item -ItemType Directory -Force -Path $runRoot | Out-Null
New-Item -ItemType Directory -Force -Path $csvRoot | Out-Null

Push-Location $botRoot
try {
    if ($BuildCurrent -or -not (Test-Path $currentExe) -or -not (Test-Path $zooExe) -or -not (Test-Path $mushroomExe)) {
        cargo build --release --bin mushroom-bot --bin zoo_bot --bin mushroom --bin mushroom_1
    }
} finally {
    Pop-Location
}

if (-not (Test-Path $weightsCurrent)) {
    Copy-Item -LiteralPath $weightsBase -Destination $weightsCurrent -Force
}

$env:MUSHROOM_USE_QSEARCH = "0"
$env:MUSHROOM_USE_NMP = "0"
$env:MUSHROOM_USE_LMR = "1"
$env:MUSHROOM_USE_EXACT_ENDGAME = "1"
$env:MUSHROOM_USE_SECOND_BONUS = "1"

if (Test-Path $mquality) {
    $env:MUSHROOM_USE_MQUALITY = "1"
    Write-Host "MQuality enabled - using existing table"
} else {
    $env:MUSHROOM_USE_MQUALITY = "0"
    Write-Host "MQuality disabled - bootstrap mode"
}

$oldPythonPath = $env:PYTHONPATH
$env:PYTHONPATH = Split-Path $botRoot -Parent

$zooModes = @(
    'greedy_area',
    'greedy_recapture',
    'greedy_fresh',
    'greedy_edge',
    'greedy_corner',
    'reply_aware',
    'reply_aware_strict',
    'defensive_when_leading',
    'defensive_when_losing',
    'pass_abuser',
    'pass_safe',
    'random_top_3',
    'random_top_5',
    'random_top_7',
    'minimax_depth_1',
    'minimax_depth_2',
    'minimax_depth_3',
    'minimax_depth_4',
    'greedy_balanced',
    'mixed_tactical',
    'endgame_expert'
)

function Invoke-ZooMatch {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Exec2,
        [Parameter(Mandatory = $true)][int]$Games,
        [int]$LocalSeed = $Seed,
        [int]$TimeBudgetMs = $BudgetMs,
        [int]$TimeoutSeconds = 0
    )

    $logFile = Join-Path $runRoot ($Name + '.log')
    $csvFile = Join-Path $csvRoot ($Name + '.csv')
    Write-Host "Running $Name ($Games games)"

    $toolArgs = @(
        $tool,
        '--games', $Games,
        '--seed', $LocalSeed,
        '--time-budget', $TimeBudgetMs,
        '--shuffle-sides',
        '--progress',
        '--log', $logFile,
        '--csv-log', $csvFile,
        '--exec1', $currentExe,
        '--exec2', $Exec2
    )

    if ($TimeoutSeconds -gt 0) {
        $job = Start-Job -ScriptBlock {
            param($args) python @args 2>&1; $LASTEXITCODE
        } -ArgumentList $toolArgs
        $job | Wait-Job -Timeout $TimeoutSeconds | Out-Null
        if ($job.State -eq 'Completed') {
            $exitCode = $job | Receive-Job
            if ($exitCode -is [int] -and $exitCode -ne 0) {
                throw "$Name failed with exit code $exitCode"
            }
        } else {
            $job | Stop-Job -PassThru | Remove-Job -Force
            throw "$Name timed out after ${TimeoutSeconds}s"
        }
    } else {
        python @toolArgs
        if ($LASTEXITCODE -ne 0) {
            throw "$Name failed with exit code $LASTEXITCODE"
        }
    }
}

function Invoke-BenchmarkMatch {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Exec2,
        [Parameter(Mandatory = $true)][int]$Games,
        [int]$LocalSeed = $BenchmarkSeed,
        [int]$TimeBudgetMs = $BenchmarkBudgetMs
    )

    Invoke-ZooMatch -Name $Name -Exec2 $Exec2 -Games $Games -LocalSeed $LocalSeed -TimeBudgetMs $TimeBudgetMs
}

function Invoke-PoolTrain {
    param(
        [Parameter(Mandatory = $true)][int]$Games,
        [int]$LocalSeed = $Seed
    )

    if (-not (Test-Path $botPool)) {
        throw "bot_pool not found: $botPool"
    }

    $poolBots = Get-ChildItem -Path $botPool -Filter *.exe -File | Sort-Object Name
    if ($poolBots.Count -eq 0) {
        throw "No .exe files found in bot_pool: $botPool"
    }

    Write-Host "=== Pool batch: bot vs bot_pool ($Games games across $($poolBots.Count) bots) ==="
    $gamesEach = [Math]::Max(1, [Math]::Ceiling($Games / $poolBots.Count))
    for ($i = 0; $i -lt $poolBots.Count; $i++) {
        $bot = $poolBots[$i]
        $botSeed = $LocalSeed + ($i * 10000)
        $modeName = "pool_$($bot.BaseName)"
        $exec2 = "`"$($bot.FullName)`""
        try {
            Invoke-ZooMatch -Name $modeName -Exec2 $exec2 -Games $gamesEach -LocalSeed $botSeed -TimeoutSeconds 600
        } catch {
            Write-Warning "Pool bot $($bot.Name) failed, skipping: $_"
        }
    }
}

if ($UsePoolTrain) {
    Invoke-PoolTrain -Games $PoolGames -LocalSeed $Seed
} else {
    Write-Host "=== Chaos batch: bot vs zoo ($TotalGames games) ==="
    $gamesPerMode = [Math]::Max(1, [Math]::Ceiling($TotalGames / $zooModes.Count))
    foreach ($mode in $zooModes) {
        $modeSeed = $Seed + ($zooModes.IndexOf($mode) * 10000)
        $modeName = "chaos_$mode"
        $exec2 = "`"$zooExe`" --mode $mode --seed $modeSeed"
        Invoke-ZooMatch -Name $modeName -Exec2 $exec2 -Games $gamesPerMode -LocalSeed $modeSeed
    }
}

Write-Host "=== Self-play batch: bot vs bot ==="
$gamesActuallyRun = if ($UsePoolTrain) { $PoolGames } else { $TotalGames }
$selfPlayGames = [Math]::Max(500, [int]($gamesActuallyRun * 0.2))
Invoke-ZooMatch -Name "selfplay" -Exec2 $currentExe -Games $selfPlayGames -LocalSeed ($Seed + 9999)

Push-Location $botRoot
$oldPythonPath = $env:PYTHONPATH
$env:PYTHONPATH = Split-Path $botRoot -Parent
try {
    Write-Host "Converting chaos CSV logs to training data: $trainingCsv"
    python .\scripts\log_to_training_data.py $csvRoot --output $trainingCsv
    if ($LASTEXITCODE -ne 0) {
        throw "log_to_training_data failed with exit code $LASTEXITCODE"
    }

    Write-Host "Updating weights from chaos logs"
    cargo run --bin update_weights -- --log-file $csvRoot --base $weightsCurrent --output $weightsCurrent
    if ($LASTEXITCODE -ne 0) {
        throw "update_weights failed with exit code $LASTEXITCODE"
    }

    Write-Host "Rebuilding MQuality from chaos logs"
    cargo run --bin gen_mquality -- --txt-dir $csvRoot --output $mquality
    if ($LASTEXITCODE -ne 0) {
        throw "gen_mquality failed with exit code $LASTEXITCODE"
    }

    Write-Host "Rebuilding data.bin"
    cargo run --bin build_data -- --geometry $geometry --weights $weightsCurrent --mquality $mquality --output $dataBin
    if ($LASTEXITCODE -ne 0) {
        throw "build_data failed with exit code $LASTEXITCODE"
    }
} finally {
    $env:PYTHONPATH = $oldPythonPath
    Pop-Location
}

if ($Benchmark) {
    if (-not (Test-Path $mushroomExe)) {
        Push-Location $botRoot
        try {
            cargo build --release --bin mushroom
        } finally {
            Pop-Location
        }
    }
    $mushroomExec = "`"$mushroomExe`""
    Invoke-BenchmarkMatch -Name 'benchmark_vs_mushroom_py' -Exec2 $mushroomExec -Games $BenchmarkGames -LocalSeed $BenchmarkSeed
    if (Test-Path $legacyExe1) {
        Invoke-BenchmarkMatch -Name 'benchmark_vs_mushroom_1' -Exec2 $legacyExe1 -Games $BenchmarkGames -LocalSeed ($BenchmarkSeed + 1)
    }
}

Write-Host "Chaos run complete. Training CSV: $trainingCsv"
