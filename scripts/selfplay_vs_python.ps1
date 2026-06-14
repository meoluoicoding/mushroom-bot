param(
    [int]$Rounds = 1,
    [int]$GamesPerRound = 1000,
    [int]$BudgetMs = 2000,
    [int]$Seed = 42,
    [bool]$ShuffleSides = $true,
    [bool]$Progress = $true,
    [int]$BenchmarkGames = 10
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$dataDir = Join-Path $root 'data'
$logsRoot = Join-Path $dataDir 'logs'
$runDir = Join-Path $logsRoot 'python_selfplay'
$weightsCurrent = Join-Path $dataDir 'current_weights.txt'
$weightsBase = Join-Path $root 'balanced.txt'
$geometry = Join-Path $dataDir 'geometry.bin'
$dataBin = Join-Path $dataDir 'data.bin'
$mquality = Join-Path $dataDir 'mquality.bin'
$botExe = Join-Path $root 'target\release\mushroom-bot.exe'
$pyBot = Join-Path $root 'target\release\mushroom.exe'

New-Item -ItemType Directory -Force -Path $runDir | Out-Null

Push-Location $root
try {
    Write-Host "Building release targets..."
    cargo build --release --bins
    if (-not (Test-Path $weightsCurrent)) {
        Copy-Item -LiteralPath $weightsBase -Destination $weightsCurrent -Force
    }

    for ($round = 1; $round -le $Rounds; $round++) {
        $roundTag = '{0:D2}' -f $round
        $csvLog = Join-Path $runDir ("round_{0}.csv" -f $roundTag)
        $textLog = Join-Path $runDir ("round_{0}.log" -f $roundTag)

        Write-Host "Round ${roundTag}: rust vs exe self-play -> $csvLog"

        $toolArgs = @(
            'testing_tool.py',
            '--games', $GamesPerRound,
            '--seed', ($Seed + $round - 1),
            '--time-budget', $BudgetMs,
            '--csv-log', $csvLog,
            '--log', $textLog,
            '--exec1', $botExe,
            '--exec2', $pyBot
        )
        if ($ShuffleSides) {
            $toolArgs += '--shuffle-sides'
        }
        if ($Progress) {
            $toolArgs += '--progress'
        } else {
            $toolArgs += '--no-progress'
        }

        python @toolArgs
        if ($LASTEXITCODE -ne 0) {
            throw "Self-play runner failed with exit code $LASTEXITCODE"
        }

        Write-Host "Round ${roundTag}: update weights from $runDir"
        cargo run --bin update_weights -- --log-file $runDir --base $weightsCurrent --output $weightsCurrent
        if ($LASTEXITCODE -ne 0) {
            throw "update_weights failed with exit code $LASTEXITCODE"
        }

        Write-Host "Round ${roundTag}: rebuild MQuality"
        cargo run --bin gen_mquality -- --txt-dir $runDir --output $mquality
        if ($LASTEXITCODE -ne 0) {
            throw "gen_mquality failed with exit code $LASTEXITCODE"
        }

        Write-Host "Round ${roundTag}: rebuild data.bin"
        cargo run --bin build_data -- --geometry $geometry --weights $weightsCurrent --mquality $mquality --output $dataBin
        if ($LASTEXITCODE -ne 0) {
            throw "build_data failed with exit code $LASTEXITCODE"
        }

        Write-Host "Round ${roundTag}: benchmark"
        cargo run --release --bin tournament -- --games $BenchmarkGames --depth-a 3 --depth-b 3 --budget $BudgetMs --swap --progress --data $dataBin --weights $weightsCurrent
        if ($LASTEXITCODE -ne 0) {
            throw "benchmark tournament failed with exit code $LASTEXITCODE"
        }
    }
}
finally {
    Pop-Location
}
