param(
    [int]$PoolGames = 2000,
    [int]$BudgetMs = 20,
    [int]$Seed = 42,
    [string]$RunTag = $(Get-Date -Format 'yyyyMMdd-HHmmss'),
    [switch]$BuildCurrent,
    [switch]$Benchmark
)

$scriptRoot = $PSScriptRoot
$runChaos = Join-Path $scriptRoot 'run_chaos.ps1'

& $runChaos -UsePoolTrain -PoolGames $PoolGames -BudgetMs $BudgetMs -Seed $Seed -RunTag $RunTag -BuildCurrent:$BuildCurrent -Benchmark:$Benchmark
