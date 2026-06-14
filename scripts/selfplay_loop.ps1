param(
    [int]$Rounds = 5,
    [int]$GamesPerRound = 50,
    [int]$DepthA = 3,
    [int]$DepthB = 3,
    [int]$BudgetMs = 20,
    [bool]$Swap = $true,
    [bool]$Progress = $true
)

$ErrorActionPreference = 'Stop'

$root = Split-Path -Parent $PSScriptRoot
$dataDir = Join-Path $root 'data'
$logsDir = Join-Path $dataDir 'logs'
$weightsCurrent = Join-Path $dataDir 'current_weights.txt'
$weightsBase = Join-Path $root 'balanced.txt'
$geometry = Join-Path $dataDir 'geometry.bin'
$dataBin = Join-Path $dataDir 'data.bin'
$mquality = Join-Path $dataDir 'mquality.bin'

New-Item -ItemType Directory -Force -Path $logsDir | Out-Null

Push-Location $root
try {
    Write-Host "Building release targets..."
    cargo build --release --bins
    if (-not (Test-Path $weightsCurrent)) {
        Copy-Item -LiteralPath $weightsBase -Destination $weightsCurrent -Force
    }

    for ($round = 1; $round -le $Rounds; $round++) {
        $roundTag = '{0:D2}' -f $round
        $logFile = Join-Path $logsDir ("round_{0}.csv" -f $roundTag)
        Write-Host "Round ${roundTag}: self-play -> $logFile"

        $tournamentArgs = @(
            'run', '--release', '--bin', 'tournament', '--',
            '--games', $GamesPerRound,
            '--depth-a', $DepthA,
            '--depth-b', $DepthB,
            '--budget', $BudgetMs,
            '--log-file', $logFile,
            '--data', $dataBin,
            '--weights', $weightsCurrent
        )
        if ($Swap) {
            $tournamentArgs += '--swap'
        }
        if ($Progress) {
            $tournamentArgs += '--progress'
        }
        cargo @tournamentArgs

        Write-Host "Round ${roundTag}: update weights from logs"
        cargo run --bin update_weights -- --log-file $logsDir --base $weightsCurrent --output $weightsCurrent

        Write-Host "Round ${roundTag}: rebuild MQuality"
        cargo run --bin gen_mquality -- --txt-dir $logsDir --output $mquality

        Write-Host "Round ${roundTag}: rebuild data.bin"
        cargo run --bin build_data -- --geometry $geometry --weights $weightsCurrent --mquality $mquality --output $dataBin

        Write-Host "Round ${roundTag}: benchmark"
        cargo run --release --bin tournament -- --games 10 --depth-a $DepthA --depth-b $DepthB --budget $BudgetMs --swap --progress --data $dataBin --weights $weightsCurrent
    }

} finally {
    Pop-Location
}
