param(
    [int]$CurrentGames = 500,
    [int]$OldGames = 2000,
    [int]$SelfPlayGames = 3000,
    [int]$BudgetMs = 20,
    [int]$Seed = 42,
    [string]$RunTag = $(Get-Date -Format 'yyyyMMdd-HHmmss'),
    [switch]$BuildCurrent
)

$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot
$tool = Join-Path $root 'mushroom-bot\testing_tool.py'
$botRoot = Join-Path $root 'mushroom-bot'
$currentExe = Join-Path $botRoot 'target\release\mushroom-bot.exe'
$zooExe = Join-Path $botRoot 'target\release\zoo_bot.exe'
$legacyExe1 = Join-Path $root 'mushroom_1.exe'
$legacyExe2 = Join-Path $root 'main.exe'
$logsRoot = Join-Path $botRoot 'data\logs\zoo'
$runRoot = Join-Path $logsRoot $RunTag
$csvRoot = Join-Path $runRoot 'csv'
$trainingCsv = Join-Path $runRoot 'training.csv'

New-Item -ItemType Directory -Force -Path $runRoot | Out-Null
New-Item -ItemType Directory -Force -Path $csvRoot | Out-Null

Push-Location $botRoot
try {
    if ($BuildCurrent -or -not (Test-Path $currentExe) -or -not (Test-Path $zooExe)) {
        cargo build --release --bin mushroom-bot --bin zoo_bot
    }
} finally {
    Pop-Location
}

$env:MUSHROOM_USE_QSEARCH = "0"
$env:MUSHROOM_USE_NMP = "0"
$env:MUSHROOM_USE_LMR = "1"
$env:MUSHROOM_USE_EXACT_ENDGAME = "1"
$env:MUSHROOM_USE_MQUALITY = "0"
$env:MUSHROOM_USE_SECOND_BONUS = "1"

function Invoke-ZooMatch {
    param(
        [Parameter(Mandatory = $true)][string]$Name,
        [Parameter(Mandatory = $true)][string]$Exec2,
        [Parameter(Mandatory = $true)][int]$Games,
        [int]$LocalSeed = $Seed
    )

    $logFile = Join-Path $logsRoot ($Name + '.log')
    $csvFile = Join-Path $csvRoot ($Name + '.csv')
    Write-Host "Running $Name ($Games games)"

    $toolArgs = @(
        $tool,
        '--games', $Games,
        '--seed', $LocalSeed,
        '--time-budget', $BudgetMs,
        '--shuffle-sides',
        '--progress',
        '--log', $logFile,
        '--csv-log', $csvFile,
        '--exec1', '.\target\release\mushroom-bot.exe',
        '--exec2', $Exec2
    )
    python @toolArgs
}

function Invoke-CurrentVsZoo {
    param(
        [Parameter(Mandatory = $true)][string]$Mode,
        [Parameter(Mandatory = $true)][int]$Games,
        [int]$LocalSeed = $Seed
    )

    Invoke-ZooMatch -Name ("current_vs_{0}" -f $Mode) -Exec2 ("`"{0}`" --mode {1} --seed {2}" -f $zooExe, $Mode, $LocalSeed) -Games $Games -LocalSeed $LocalSeed
}

Invoke-CurrentVsZoo -Mode 'greedy_area' -Games $CurrentGames -LocalSeed ($Seed + 1)
Invoke-CurrentVsZoo -Mode 'greedy_recapture' -Games $CurrentGames -LocalSeed ($Seed + 2)
Invoke-CurrentVsZoo -Mode 'reply_aware' -Games $CurrentGames -LocalSeed ($Seed + 3)
Invoke-CurrentVsZoo -Mode 'defensive_when_leading' -Games $CurrentGames -LocalSeed ($Seed + 4)
Invoke-CurrentVsZoo -Mode 'pass_abuser' -Games $CurrentGames -LocalSeed ($Seed + 5)

$legacyBots = @()
if (Test-Path $legacyExe1) {
    $legacyBots += @{ Name = 'mushroom_1'; Exec = '..\mushroom_1.exe' }
}
if (Test-Path $legacyExe2) {
    $legacyBots += @{ Name = 'main'; Exec = '..\main.exe' }
}

if ($legacyBots.Count -eq 0) {
    Write-Warning "No legacy executable found. Compile main.cpp to main.exe if you want it in current_vs_old_versions."
} else {
    $share = [Math]::Max(1, [int][Math]::Floor($OldGames / $legacyBots.Count))
    $leftover = $OldGames - ($share * $legacyBots.Count)
    for ($i = 0; $i -lt $legacyBots.Count; $i++) {
        $games = $share
        if ($i -lt $leftover) {
            $games++
        }
        $bot = $legacyBots[$i]
        Invoke-ZooMatch -Name ("current_vs_old_{0}" -f $bot.Name) -Exec2 $bot.Exec -Games $games -LocalSeed ($Seed + 100 + $i)
    }
}

Invoke-ZooMatch -Name 'current_selfplay' -Exec2 '.\target\release\mushroom-bot.exe' -Games $SelfPlayGames -LocalSeed ($Seed + 999)

Push-Location $root
$oldPythonPath = $env:PYTHONPATH
$env:PYTHONPATH = $root
try {
    Write-Host "Converting CSV logs to training data: $trainingCsv"
    python .\mushroom-bot\log_to_training_data.py $csvRoot --output $trainingCsv
} finally {
    $env:PYTHONPATH = $oldPythonPath
    Pop-Location
}
