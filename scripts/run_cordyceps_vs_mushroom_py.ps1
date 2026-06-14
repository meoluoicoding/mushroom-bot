param(
    [string]$CordycepsWrapper = ".\cordyceps_wrapper.py",
    [string]$MushroomExe = ".\target\release\mushroom.exe",
    [int]$Games = 20,
    [int]$TimeBudgetMs = 10000,
    [int]$Seed = 42,
    [string]$LogFile = ".\log_cordyceps_vs_mushroom_py.txt",
    [string]$CsvLogFile = ".\benchmark_cordyceps_vs_mushroom_py.csv",
    [string]$InputFile = "",
    [switch]$ShuffleSides,
    [switch]$DebugWrapper
)

$ErrorActionPreference = "Stop"

$Root = Split-Path -Parent $MyInvocation.MyCommand.Path
$TestingTool = Join-Path $Root "testing_tool.py"

if (-not (Test-Path -LiteralPath $TestingTool)) {
    throw "Missing testing_tool.py at $TestingTool"
}

$CordycepsWrapperCandidate = Join-Path $Root $CordycepsWrapper
if (-not (Test-Path -LiteralPath $CordycepsWrapperCandidate)) {
    throw "Missing Cordyceps wrapper: $CordycepsWrapperCandidate"
}
$CordycepsWrapper = (Resolve-Path $CordycepsWrapperCandidate).Path

$MushroomExeCandidate = Join-Path $Root $MushroomExe
if (-not (Test-Path -LiteralPath $MushroomExeCandidate)) {
    throw "Missing mushroom.exe: $MushroomExeCandidate"
}
$MushroomExe = (Resolve-Path $MushroomExeCandidate).Path

$LogFilePath = if ([System.IO.Path]::IsPathRooted($LogFile)) { $LogFile } else { Join-Path $Root $LogFile }
$CsvLogFilePath = if ($CsvLogFile -and [System.IO.Path]::IsPathRooted($CsvLogFile)) { $CsvLogFile } elseif ($CsvLogFile) { Join-Path $Root $CsvLogFile } else { "" }
$InputFilePath = if ($InputFile) { if ([System.IO.Path]::IsPathRooted($InputFile)) { $InputFile } else { Join-Path $Root $InputFile } } else { "" }

$exec1 = "python -u $CordycepsWrapper"
$exec2 = $MushroomExe

$args = @(
    $TestingTool,
    "--exec1", $exec1,
    "--exec2", $exec2,
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

if ($DebugWrapper) {
    $env:CORDYCEPS_WRAPPER_DEBUG = "1"
} else {
    Remove-Item Env:CORDYCEPS_WRAPPER_DEBUG -ErrorAction SilentlyContinue
}

python @args
