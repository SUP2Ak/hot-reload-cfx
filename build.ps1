function Check-RequiredTargets {
    $requiredTargets = @("x86_64-pc-windows-msvc", "x86_64-unknown-linux-musl", "x86_64-apple-darwin")
    $missingTargets = @()

    foreach ($target in $requiredTargets) {
        $installed = rustup target list | Select-String "^$target" | Select-String "installed"
        if (-not $installed) {
            $missingTargets += $target
        }
    }

    if ($missingTargets.Count -gt 0) {
        Write-Host "❌ Missing required targets:" -ForegroundColor Red
        $missingTargets | ForEach-Object { Write-Host "   - $_" -ForegroundColor Yellow }
        Write-Host "`nPlease run install-targets.ps1 first to install missing targets." -ForegroundColor Cyan
        return $false
    }

    return $true
}

# Check targets before continuing
if (-not (Check-RequiredTargets)) {
    exit 1
}

function Create-ReleaseFolder {
    $releasePath = "release"
    if (Test-Path $releasePath) {
        Remove-Item -Path $releasePath -Recurse -Force
    }
    New-Item -Path $releasePath -ItemType Directory | Out-Null
    Write-Host "📁 Release folder created" -ForegroundColor Green
}

function Build-UI {
    Write-Host "`n🎨 Building UI..." -ForegroundColor Cyan
    
    $targets = @("x86_64-pc-windows-msvc", "x86_64-unknown-linux-musl", "x86_64-apple-darwin")
    $progress = 0
    $totalTargets = $targets.Count

    foreach ($target in $targets) {
        $progress++
        $percent = [math]::Round(($progress / $totalTargets) * 100)
        Write-Progress -Activity "Building UI" -Status "Target: $target" -PercentComplete $percent

        Write-Host "   🔨 Building for $target..." -ForegroundColor Yellow
        $buildResult = cargo build --release --target $target -p hot-reload-ui
        if ($LASTEXITCODE -ne 0) {
            Write-Host "   ❌ Build failed for $target" -ForegroundColor Red
            continue
        }

        $binName = if ($target -like "*windows*") { "hot-reload-ui.exe" } else { "hot-reload-ui" }
        $sourcePath = "target/$target/release/$binName"
        
        if (Test-Path $sourcePath) {
            $zipName = "release/hot-reload-ui_$target.zip"
            Compress-Archive -Path $sourcePath -DestinationPath $zipName -Force
            Write-Host "   ✅ Created $zipName" -ForegroundColor Green
        } else {
            Write-Host "   ❌ Binary not found: $sourcePath" -ForegroundColor Red
        }
    }
    Write-Progress -Activity "Building UI" -Completed
}

function Install-CrossIfNeeded {
    if (-not (Get-Command "cross" -ErrorAction SilentlyContinue)) {
        Write-Host "📥 Installing cross-rs..." -ForegroundColor Yellow
        
        # Vérifier si Docker est installé
        if (-not (Get-Command "docker" -ErrorAction SilentlyContinue)) {
            Write-Host "❌ Docker is required but not installed." -ForegroundColor Red
            Write-Host "Please install Docker Desktop from: https://www.docker.com/products/docker-desktop" -ForegroundColor Yellow
            return $false
        }
        
        # Installer cross
        cargo install cross
        if ($LASTEXITCODE -ne 0) {
            Write-Host "❌ Failed to install cross" -ForegroundColor Red
            return $false
        }
        
        Write-Host "✅ cross-rs installed successfully" -ForegroundColor Green
    }
    return $true
}

function Build-Watcher {
    Write-Host "`n👀 Building Watcher..." -ForegroundColor Cyan
    
    #Configurer l'environnement pour MUSL
    $muslPath = "C:\musl\tools\bin"
    if (Test-Path $muslPath) {
        $env:PATH = "$muslPath;$env:PATH"
        $env:CC_x86_64_unknown_linux_musl = "x86_64-linux-musl-gcc"
        $env:CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = "x86_64-linux-musl-gcc"
    }
    
    $targets = @("x86_64-pc-windows-msvc", "x86_64-unknown-linux-musl")
    $progress = 0
    $totalTargets = $targets.Count

    foreach ($target in $targets) {
        $progress++
        $percent = [math]::Round(($progress / $totalTargets) * 100)
        Write-Progress -Activity "Building Watcher" -Status "Target: $target" -PercentComplete $percent

        Write-Host "   🔨 Building for $target..." -ForegroundColor Yellow
        
        # Utiliser cross pour Linux
        if ($target -eq "x86_64-unknown-linux-musl") {
            # Vérifier/Installer cross si nécessaire
            if (-not (Install-CrossIfNeeded)) {
                Write-Host "   ❌ Build failed for $target : cross-rs not available" -ForegroundColor Red
                continue
            }
            
            # Utiliser le chemin complet de cross
            $crossPath = Join-Path $env:USERPROFILE ".cargo\bin\cross.exe"
            $buildResult = & $crossPath build --release --target $target -p hot-reload-watcher
        } else {
            $buildResult = cargo build --release --target $target -p hot-reload-watcher
        }
        
        if ($LASTEXITCODE -ne 0) {
            Write-Host "   ❌ Build failed for $target" -ForegroundColor Red
            continue
        }

        $binName = if ($target -like "*windows*") { "hot-reload-watcher.exe" } else { "hot-reload-watcher" }
        $sourcePath = "target/$target/release/$binName"
        
        if (Test-Path $sourcePath) {
            $tempDir = "temp_watcher"
            New-Item -Path $tempDir -ItemType Directory -Force | Out-Null
            Copy-Item $sourcePath -Destination $tempDir
            Copy-Item "config.json" -Destination $tempDir -ErrorAction SilentlyContinue

            $zipName = "release/hot-reload-watcher_$target.zip"
            Compress-Archive -Path "$tempDir/*" -DestinationPath $zipName -Force
            Remove-Item -Path $tempDir -Recurse -Force
            Write-Host "   ✅ Created $zipName" -ForegroundColor Green
        }
    }
    Write-Progress -Activity "Building Watcher" -Completed
}

function Build-FXServer {
    Write-Host "`n🎮 Building FX Server Resource..." -ForegroundColor Cyan
    
    Write-Progress -Activity "Building FX Server" -Status "Running pnpm build" -PercentComplete 25
    
    Push-Location resources/hot-reload
    $buildResult = pnpm build
    if ($LASTEXITCODE -ne 0) {
        Write-Host "   ❌ FX Server build failed" -ForegroundColor Red
        Pop-Location
        Write-Progress -Activity "Building FX Server" -Completed
        return
    }
    Pop-Location

    Write-Progress -Activity "Building FX Server" -Status "Creating archive" -PercentComplete 75

    $tempDir = "temp_fxserver/hot-reload"
    New-Item -Path $tempDir -ItemType Directory -Force | Out-Null

    Copy-Item "resources/hot-reload/dist" -Destination $tempDir -Recurse
    Copy-Item "resources/hot-reload/fxmanifest.lua" -Destination $tempDir

    Compress-Archive -Path "temp_fxserver/*" -DestinationPath "release/hot-reload-fxserver.zip" -Force
    Remove-Item -Path "temp_fxserver" -Recurse -Force
    Write-Host "   ✅ Created hot-reload-fxserver.zip" -ForegroundColor Green
    
    Write-Progress -Activity "Building FX Server" -Completed
}

function Show-Menu {
    Write-Host "`n🚀 Hot Reload Builder" -ForegroundColor Magenta
    Write-Host "------------------------"
    Write-Host "1. UI (Windows, Linux, MacOS)"
    Write-Host "2. Watcher (Windows, Linux)"
    Write-Host "3. FX Server Resource"
    Write-Host "4. All"
    Write-Host "5. Quit"
    Write-Host "------------------------"
}

# Main
Create-ReleaseFolder

do {
    Show-Menu
    $choice = Read-Host "Choose an option"

    switch ($choice) {
        "1" { Build-UI }
        "2" { Build-Watcher }
        "3" { Build-FXServer }
        "4" { 
            Build-UI
            Build-Watcher
            Build-FXServer
        }
        "5" { 
            Write-Host "`n👋 Bye!" -ForegroundColor Cyan
            exit 
        }
        default { Write-Host "`n❌ Invalid option" -ForegroundColor Red }
    }

    Write-Host "`nPress a key to continue..."
    $null = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
    Clear-Host

} while ($true)