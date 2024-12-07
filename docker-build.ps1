function Check-DockerInstallation {
    if (-not (Get-Command "docker" -ErrorAction SilentlyContinue)) {
        Write-Host "❌ Docker is not installed!" -ForegroundColor Red
        Write-Host "Please install Docker Desktop from: https://www.docker.com/products/docker-desktop" -ForegroundColor Yellow
        return $false
    }
    
    try {
        docker info | Out-Null
    } catch {
        Write-Host "❌ Docker is not running!" -ForegroundColor Red
        Write-Host "Please start Docker Desktop" -ForegroundColor Yellow
        return $false
    }
    
    return $true
}

function Build-DockerImage {
    param (
        [string]$platform
    )
    
    $dockerfile = @"
FROM rust:latest
RUN rustup target add x86_64-unknown-linux-musl
RUN rustup target add x86_64-apple-darwin
RUN rustup target add aarch64-apple-darwin
RUN cargo install cross
WORKDIR /app
"@
    
    Write-Host "🐳 Building Docker image for $platform..." -ForegroundColor Cyan
    $dockerfile | Out-File -FilePath "Dockerfile" -Encoding utf8
    docker build -t hot-reload-builder .
    Remove-Item "Dockerfile"
}

function Show-BuildMenu {
    Write-Host "`n🚀 Docker Build Menu" -ForegroundColor Cyan
    Write-Host "------------------------"
    Write-Host "1. Build UI (All platforms)"
    Write-Host "2. Build Watcher (All platforms)"
    Write-Host "3. Build Everything"
    Write-Host "4. Exit"
    Write-Host "------------------------"
    
    $choice = Read-Host "Choose an option"
    return $choice
}

function Build-Project {
    param (
        [string]$project,
        [string]$target
    )
    
    Write-Host "`n🔨 Building $project for $target..." -ForegroundColor Yellow
    
    $volumePath = (Get-Location).Path
    $dockerCmd = "docker run --rm -v `"${volumePath}:/app`" hot-reload-builder"
    $buildCmd = "cross build --release --target $target -p $project"
    
    Invoke-Expression "$dockerCmd $buildCmd"
    
    if ($LASTEXITCODE -eq 0) {
        $binName = if ($target -like "*windows*") { "$project.exe" } else { $project }
        $sourcePath = "target/$target/release/$binName"
        
        if (Test-Path $sourcePath) {
            $tempDir = "temp_build"
            New-Item -Path $tempDir -ItemType Directory -Force | Out-Null
            Copy-Item $sourcePath -Destination $tempDir
            Copy-Item "config.json" -Destination $tempDir -ErrorAction SilentlyContinue
            
            $zipName = "release/${project}_${target}.zip"
            Compress-Archive -Path "$tempDir/*" -DestinationPath $zipName -Force
            Remove-Item -Path $tempDir -Recurse -Force
            Write-Host "   ✅ Created $zipName" -ForegroundColor Green
        }
    } else {
        Write-Host "   ❌ Build failed for $target" -ForegroundColor Red
    }
}

function Build-AllPlatforms {
    param (
        [string]$project
    )
    
    $targets = @(
        "x86_64-pc-windows-msvc",
        "x86_64-unknown-linux-musl",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin"
    )
    
    foreach ($target in $targets) {
        Build-Project -project $project -target $target
    }
}

# Main
Clear-Host
Write-Host "🐳 Docker Build System" -ForegroundColor Magenta
Write-Host "------------------------"

if (-not (Check-DockerInstallation)) {
    exit 1
}

New-Item -Path "release" -ItemType Directory -Force | Out-Null

Build-DockerImage

do {
    $choice = Show-BuildMenu
    
    switch ($choice) {
        "1" { 
            Build-AllPlatforms -project "hot-reload-ui"
        }
        "2" { 
            Build-AllPlatforms -project "hot-reload-watcher"
        }
        "3" { 
            Build-AllPlatforms -project "hot-reload-ui"
            Build-AllPlatforms -project "hot-reload-watcher"
        }
        "4" { 
            Write-Host "`n👋 Goodbye!" -ForegroundColor Cyan
            exit 0
        }
        default {
            Write-Host "❌ Invalid option" -ForegroundColor Red
        }
    }
    
    Write-Host "`nPress any key to continue..."
    $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
    Clear-Host
} while ($true)