param(
    [string]$RustBotExe = ".\target\release\mushroom-bot.exe",
    [string]$MushroomExe = ".\target\release\mushroom.exe",
    [int]$Games = 50,
    [int]$TimeBudgetMs = 10000,
    [int]$Seed = 42,
    [string]$LogFile = ".\log_eval_vs_mushroom_py.txt",
    [string]$CsvLogFile = ".\eval_vs_mushroom_py.csv",
    [switch]$BuildCurrent,
    [switch]$ShuffleSides = $true
)

$ErrorActionPreference = "Stop"

$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$BotRoot = Split-Path $ScriptRoot -Parent
$TestingTool = Join-Path $ScriptRoot "testing_tool.py"

if (-not (Test-Path -LiteralPath $TestingTool)) {
    throw "Missing testing_tool.py at $TestingTool"
}

$RustBotCandidate = Join-Path $BotRoot $RustBotExe
if (-not (Test-Path -LiteralPath $RustBotCandidate)) {
    throw "Missing mushroom-bot exe: $RustBotCandidate"
}
$RustBotExe = (Resolve-Path $RustBotCandidate).Path

$MushroomExeCandidate = Join-Path $BotRoot $MushroomExe
if (-not (Test-Path -LiteralPath $MushroomExeCandidate)) {
    throw "Missing mushroom.exe: $MushroomExeCandidate"
}
$MushroomExe = (Resolve-Path $MushroomExeCandidate).Path

$LogFilePath = if ([System.IO.Path]::IsPathRooted($LogFile)) { $LogFile } else { Join-Path $BotRoot $LogFile }
$CsvLogFilePath = if ($CsvLogFile -and [System.IO.Path]::IsPathRooted($CsvLogFile)) { $CsvLogFile } elseif ($CsvLogFile) { Join-Path $BotRoot $CsvLogFile } else { "" }

if ($BuildCurrent) {
    Push-Location $BotRoot
    try {
        cargo build --release --bin mushroom-bot
    } finally {
        Pop-Location
    }
}

$args = @(
    $TestingTool,
    "--exec1", $RustBotExe,
    "--exec2", $MushroomExe,
    "--games", $Games,
    "--time-budget", $TimeBudgetMs,
    "--seed", $Seed,
    "--log", $LogFilePath,
    "--progress"
)

if ($CsvLogFilePath) {
    $args += @("--csv-log", $CsvLogFilePath)
}

if ($ShuffleSides) {
    $args += "--shuffle-sides"
}

Push-Location $BotRoot
try {
    $prevErrorActionPreference = $ErrorActionPreference
    $ErrorActionPreference = "Continue"
    try {
        $output = & python @args 2>&1
    } finally {
        $ErrorActionPreference = $prevErrorActionPreference
    }
    $output | ForEach-Object { Write-Host $_ }

    $summary = $output | Where-Object { $_ -match '^Games:\s+\d+\s+\|.*BOT1=(\d+)\s+BOT2=(\d+)\s+DRAW=(\d+)' } | Select-Object -Last 1
    if ($summary -match '^Games:\s+\d+\s+\|.*BOT1=(\d+)\s+BOT2=(\d+)\s+DRAW=(\d+)') {
        $bot1 = [int]$matches[1]
        $bot2 = [int]$matches[2]
        $draw = [int]$matches[3]
        $total = [Math]::Max(1, $bot1 + $bot2 + $draw)
        $winrate = [Math]::Round(($bot1 / $total) * 100.0, 2)
        Write-Host ("Winrate BOT1={0}/{1} = {2}%" -f $bot1, $total, $winrate)
    } else {
        Write-Warning "Could not parse final summary line."
    }
    if ($LASTEXITCODE -ne 0) {
        throw "evaluation failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
}
