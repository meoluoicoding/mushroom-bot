param(
    [string]$LogsRoot = ".\data\logs\chaos",
    [int]$Limit = 20,
    [string]$ExportCsv = ""
)

$ErrorActionPreference = 'Stop'

$ScriptRoot = Split-Path -Parent $MyInvocation.MyCommand.Path
$BotRoot = Split-Path $ScriptRoot -Parent
$LogsRootPath = if ([System.IO.Path]::IsPathRooted($LogsRoot)) { $LogsRoot } else { Join-Path $BotRoot $LogsRoot }

if (-not (Test-Path -LiteralPath $LogsRootPath)) {
    throw "Missing logs root: $LogsRootPath"
}

function Parse-Summary {
    param([string]$Path)

    $last = Select-String -Path $Path -Pattern '^SUMMARY BOT1=(\d+) BOT2=(\d+) DRAW=(\d+)' | Select-Object -Last 1
    if (-not $last) {
        return $null
    }

    if ($last.Line -match '^SUMMARY BOT1=(\d+) BOT2=(\d+) DRAW=(\d+)') {
        $bot1 = [int]$matches[1]
        $bot2 = [int]$matches[2]
        $draw = [int]$matches[3]
        $total = [Math]::Max(1, $bot1 + $bot2 + $draw)
        $winrate = [Math]::Round(($bot1 / $total) * 100.0, 2)
        return [pscustomobject]@{
            Path = $Path
            Bot1 = $bot1
            Bot2 = $bot2
            Draw = $draw
            Total = $total
            WinRate = $winrate
        }
    }

    return $null
}

$rows = Get-ChildItem -Recurse -File $LogsRootPath -Filter benchmark_vs_mushroom_*.log |
    Sort-Object LastWriteTime |
    ForEach-Object {
        $summary = Parse-Summary -Path $_.FullName
        if ($null -ne $summary) {
            $summary | Add-Member -NotePropertyName RunTag -NotePropertyValue $_.Directory.Parent.Name -Force
            $summary | Add-Member -NotePropertyName Name -NotePropertyValue $_.BaseName -Force
            $summary
        }
    }

if (-not $rows) {
    Write-Host "No benchmark logs found under $LogsRootPath"
    exit 0
}

$recent = $rows | Select-Object -Last ([Math]::Min($Limit, $rows.Count))

Write-Host "Recent benchmark history"
Write-Host ("{0,-20} {1,-28} {2,6} {3,6} {4,6} {5,6} {6,8}" -f "run_tag", "name", "bot1", "bot2", "draw", "total", "winrate")
foreach ($row in $recent) {
    Write-Host ("{0,-20} {1,-28} {2,6} {3,6} {4,6} {5,6} {6,7}%" -f $row.RunTag, $row.Name, $row.Bot1, $row.Bot2, $row.Draw, $row.Total, $row.WinRate)
}

$grouped = $rows | Group-Object Name
Write-Host ""
Write-Host "Averages by opponent"
Write-Host ("{0,-28} {1,6} {2,6} {3,6} {4,8}" -f "name", "games", "bot1", "draw", "winrate")
foreach ($group in $grouped | Sort-Object Name) {
    $sumBot1 = ($group.Group | Measure-Object -Property Bot1 -Sum).Sum
    $sumBot2 = ($group.Group | Measure-Object -Property Bot2 -Sum).Sum
    $sumDraw = ($group.Group | Measure-Object -Property Draw -Sum).Sum
    $sumTotal = [Math]::Max(1, $sumBot1 + $sumBot2 + $sumDraw)
    $rate = [Math]::Round(($sumBot1 / $sumTotal) * 100.0, 2)
    Write-Host ("{0,-28} {1,6} {2,6} {3,6} {4,7}%" -f $group.Name, $sumTotal, $sumBot1, $sumDraw, $rate)
}

if ($ExportCsv) {
    $exportPath = if ([System.IO.Path]::IsPathRooted($ExportCsv)) { $ExportCsv } else { Join-Path $BotRoot $ExportCsv }
    $recent | Export-Csv -NoTypeInformation -Encoding UTF8 -LiteralPath $exportPath
    Write-Host ""
    Write-Host "Exported recent history to $exportPath"
}
