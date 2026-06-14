param(
    [string]$RustBotExe = ".\target\release\mushroom-bot.exe",
    [string]$MushroomExe = ".\target\release\mushroom.exe",
    [int]$Games = 50,
    [int]$TimeBudgetMs = 10000,
    [int]$Seed = 42,
    [string]$LogFile = ".\log_mushroom_bot_vs_mushroom_py.txt",
    [string]$CsvLogFile = ".\benchmark_mushroom_bot_vs_mushroom_py.csv",
    [string]$InputFile = "",
    [switch]$ShuffleSides
)

$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$TestingTool = Join-Path $Root "testing_tool.py"

if (-not (Test-Path -LiteralPath $TestingTool)) {
    throw "Missing testing_tool.py at $TestingTool"
}

$RustBotCandidate = Join-Path $Root $RustBotExe
if (-not (Test-Path -LiteralPath $RustBotCandidate)) {
    throw "Missing mushroom-bot exe: $RustBotCandidate"
}
$RustBotExe = (Resolve-Path $RustBotCandidate).Path

$MushroomExeCandidate = Join-Path $Root $MushroomExe
if (-not (Test-Path -LiteralPath $MushroomExeCandidate)) {
    throw "Missing mushroom.exe: $MushroomExeCandidate"
}
$MushroomExe = (Resolve-Path $MushroomExeCandidate).Path

$LogFilePath = if ([System.IO.Path]::IsPathRooted($LogFile)) { $LogFile } else { Join-Path $Root $LogFile }
$CsvLogFilePath = if ($CsvLogFile -and [System.IO.Path]::IsPathRooted($CsvLogFile)) { $CsvLogFile } elseif ($CsvLogFile) { Join-Path $Root $CsvLogFile } else { "" }
$InputFilePath = if ($InputFile) { if ([System.IO.Path]::IsPathRooted($InputFile)) { $InputFile } else { Join-Path $Root $InputFile } } else { "" }

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

if ($InputFilePath) {
    $args += @("--input", $InputFilePath)
}

if ($ShuffleSides) {
    $args += "--shuffle-sides"
}

Push-Location $Root
try {
    python @args
} finally {
    Pop-Location
}
