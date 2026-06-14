param(
    [string]$RunTag = $(Get-Date -Format 'yyyyMMdd-HHmmss'),
    [switch]$BuildCurrent,
    [switch]$Benchmark
)

$ErrorActionPreference = 'Stop'

$root = $PSScriptRoot
$botRoot = Join-Path $root 'mushroom-bot'
$dataDir = Join-Path $botRoot 'data'
$logsRoot = Join-Path $dataDir 'logs'
$cleanRoot = Join-Path $logsRoot 'clean'
$runRoot = Join-Path $cleanRoot $RunTag
$cleanCsvRoot = Join-Path $runRoot 'csv'
$trainingCsv = Join-Path $runRoot 'training.csv'
$currentExe = Join-Path $botRoot 'target\release\mushroom-bot.exe'
$weightsCurrent = Join-Path $dataDir 'current_weights.txt'
$weightsBase = Join-Path $root 'balanced.txt'
$geometry = Join-Path $dataDir 'geometry.bin'
$dataBin = Join-Path $dataDir 'data.bin'
$mquality = Join-Path $dataDir 'mquality.bin'

New-Item -ItemType Directory -Force -Path $cleanCsvRoot | Out-Null

Push-Location $botRoot
try {
    if ($BuildCurrent -or -not (Test-Path $currentExe)) {
        cargo build --release --bin mushroom-bot
    }
} finally {
    Pop-Location
}

if (-not (Test-Path $weightsCurrent)) {
    Copy-Item -LiteralPath $weightsBase -Destination $weightsCurrent -Force
}

$sourceRoots = @(
    (Join-Path $logsRoot 'chaos'),
    (Join-Path $logsRoot 'zoo_full'),
    (Join-Path $logsRoot 'zoo'),
    (Join-Path $logsRoot 'python_selfplay'),
    (Join-Path $logsRoot 'codex_round_01')
)

function Should-SkipPath {
    param([string]$Path)
    $name = Split-Path $Path -Leaf
    if ($name -match '^smoke') { return $true }
    if ($name -match 'smoke') { return $true }
    if ($name -eq 'training.csv') { return $true }
    if ($name -match '^bench') { return $true }
    if ($name -match '^benchmark') { return $true }
    if ($name -match '^depth_check') { return $true }
    if ($name -match '^maxply_check') { return $true }
    if ($name -match '^log_') { return $true }
    if ($name -match '^rust_vs_') { return $true }
    if ($name -match '^cli_') { return $true }
    if ($name -match '^match_') { return $true }
    if ($name -match '^selfplay\.txt$') { return $true }
    return $false
}

function Copy-CleanCsv {
    param(
        [string]$SourceRoot,
        [string]$Prefix
    )

    if (-not (Test-Path $SourceRoot)) {
        return
    }

    switch ($Prefix) {
        'chaos' {
            Get-ChildItem -Path $SourceRoot -Directory | ForEach-Object {
                $csvDir = Join-Path $_.FullName 'csv'
                if (Test-Path $csvDir) {
                    Get-ChildItem -Path $csvDir -File -Filter *.csv | ForEach-Object {
                        if (Should-SkipPath $_.FullName) { return }
                        $dest = Join-Path $cleanCsvRoot ("$Prefix" + '__' + $_.Directory.Parent.Name + '__' + $_.Name)
                        Copy-Item -LiteralPath $_.FullName -Destination $dest -Force
                    }
                }
            }
        }
        'zoo_full' {
            Get-ChildItem -Path $SourceRoot -Directory | ForEach-Object {
                $csvDir = Join-Path $_.FullName 'csv'
                if (Test-Path $csvDir) {
                    Get-ChildItem -Path $csvDir -File -Filter *.csv | ForEach-Object {
                        if (Should-SkipPath $_.FullName) { return }
                        $dest = Join-Path $cleanCsvRoot ("$Prefix" + '__' + $_.Directory.Parent.Name + '__' + $_.Name)
                        Copy-Item -LiteralPath $_.FullName -Destination $dest -Force
                    }
                }
            }
        }
        'zoo' {
            Get-ChildItem -Path $SourceRoot -Directory | Where-Object { $_.Name -notmatch '^smoke' -and $_.Name -ne 'csv' } | ForEach-Object {
                $csvDir = Join-Path $_.FullName 'csv'
                if (Test-Path $csvDir) {
                    Get-ChildItem -Path $csvDir -File -Filter *.csv | ForEach-Object {
                        if (Should-SkipPath $_.FullName) { return }
                        $dest = Join-Path $cleanCsvRoot ("$Prefix" + '__' + $_.Directory.Parent.Name + '__' + $_.Name)
                        Copy-Item -LiteralPath $_.FullName -Destination $dest -Force
                    }
                }
            }
        }
        default {
            Get-ChildItem -Path $SourceRoot -File -Filter *.csv | ForEach-Object {
                if (Should-SkipPath $_.FullName) { return }
                $dest = Join-Path $cleanCsvRoot ("$Prefix" + '__' + $_.Name)
                Copy-Item -LiteralPath $_.FullName -Destination $dest -Force
            }
        }
    }
}

foreach ($source in $sourceRoots) {
    $prefix = Split-Path $source -Leaf
    Copy-CleanCsv -SourceRoot $source -Prefix $prefix
}

Push-Location $root
$oldPythonPath = $env:PYTHONPATH
$env:PYTHONPATH = $botRoot
try {
    Write-Host "Building clean training CSV: $trainingCsv"
    python .\mushroom-bot\scripts\log_to_training_data.py $cleanCsvRoot --output $trainingCsv
    if ($LASTEXITCODE -ne 0) {
        throw "log_to_training_data failed with exit code $LASTEXITCODE"
    }
} finally {
    $env:PYTHONPATH = $oldPythonPath
    Pop-Location
}

Push-Location $botRoot
try {
    Write-Host "Updating weights from clean logs"
    cargo run --bin update_weights -- --log-file $cleanCsvRoot --base $weightsCurrent --output $weightsCurrent
    if ($LASTEXITCODE -ne 0) {
        throw "update_weights failed with exit code $LASTEXITCODE"
    }

    Write-Host "Rebuilding MQuality from clean logs"
    cargo run --bin gen_mquality -- --txt-dir $cleanCsvRoot --output $mquality
    if ($LASTEXITCODE -ne 0) {
        throw "gen_mquality failed with exit code $LASTEXITCODE"
    }

    Write-Host "Rebuilding data.bin from clean logs"
    cargo run --bin build_data -- --geometry $geometry --weights $weightsCurrent --mquality $mquality --output $dataBin
    if ($LASTEXITCODE -ne 0) {
        throw "build_data failed with exit code $LASTEXITCODE"
    }

    if ($Benchmark) {
        Write-Host "Benchmarking updated bot"
        cargo run --release --bin tournament -- --games 10 --depth-a 3 --depth-b 3 --budget 20 --swap --progress --data $dataBin --weights $weightsCurrent
        if ($LASTEXITCODE -ne 0) {
            throw "benchmark tournament failed with exit code $LASTEXITCODE"
        }
    }
} finally {
    Pop-Location
}

Write-Host "Clean rebuild complete. Training CSV: $trainingCsv"
