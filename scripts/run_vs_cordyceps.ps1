param(
    [string]$RustBotExe = ".\target\release\mushroom-bot.exe",
    [string]$CordycepsWrapper = ".\cordyceps_wrapper.py",
    [int]$Games = 20,
    [int]$TimeBudgetMs = 10000,
    [int]$Seed = 42,
    [string]$LogFile = ".\log_vs_cordyceps.txt",
    [string]$CsvLogFile = ".\log_vs_cordyceps.csv",
    [string]$InputFile = "",
    [switch]$DebugWrapper,
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
    throw "Missing Rust bot exe: $RustBotCandidate"
}
$RustBotExe = (Resolve-Path $RustBotCandidate).Path

$CordycepsWrapperCandidate = Join-Path $Root $CordycepsWrapper
if (-not (Test-Path -LiteralPath $CordycepsWrapperCandidate)) {
    throw "Missing Cordyceps wrapper: $CordycepsWrapperCandidate"
}
$CordycepsWrapper = (Resolve-Path $CordycepsWrapperCandidate).Path

$LogFilePath = if ([System.IO.Path]::IsPathRooted($LogFile)) {
    $LogFile
} else {
    (Join-Path $Root $LogFile)
}

$CsvLogFilePath = if ($CsvLogFile -and [System.IO.Path]::IsPathRooted($CsvLogFile)) {
    $CsvLogFile
} elseif ($CsvLogFile) {
    (Join-Path $Root $CsvLogFile)
} else {
    ""
}

$InputFilePath = if ($InputFile) {
    if ([System.IO.Path]::IsPathRooted($InputFile)) {
        $InputFile
    } else {
        (Join-Path $Root $InputFile)
    }
} else {
    ""
}

$args = @(
    $TestingTool,
    "--exec1", $RustBotExe,
    "--exec2", "python -u $CordycepsWrapper",
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
