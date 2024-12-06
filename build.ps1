function Show-TargetMenu {
    Write-Host "`n📦 Targets installation selection" -ForegroundColor Cyan
    Write-Host "------------------------"
    Write-Host "1. Windows (x86_64-pc-windows-msvc)"
    Write-Host "2. Linux (x86_64-unknown-linux-gnu)"
    Write-Host "3. MacOS (x86_64-apple-darwin)"
    Write-Host "4. All targets"
    Write-Host "5. Back"
    Write-Host "------------------------"

    $choice = Read-Host "Choose an option"
    
    $targets = @()
    switch ($choice) {
        "1" { $targets = @("x86_64-pc-windows-msvc") }
        "2" { $targets = @("x86_64-unknown-linux-gnu") }
        "3" { $targets = @("x86_64-apple-darwin") }
        "4" { $targets = @("x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu", "x86_64-apple-darwin") }
        "5" { return $null }
        default { 
            Write-Host "❌ Invalid option" -ForegroundColor Red
            return $null
        }
    }
    return $targets
}

function Install-RustTargets {
    Write-Host "`n🔧 Rust targets management..." -ForegroundColor Cyan
    
    $targets = Show-TargetMenu
    if ($null -eq $targets) {
        return $false
    }

    foreach ($target in $targets) {
        $installed = rustup target list | Select-String "^$target" | Select-String "installed"
        if (-not $installed) {
            Write-Host "   📥 Installing target $target..." -ForegroundColor Yellow
            
            # Simple animation during installation
            $spinner = @('⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏')
            $job = Start-Job -ScriptBlock {
                param($target)
                rustup target add $target
            } -ArgumentList $target

            $i = 0
            while ($job.State -eq 'Running') {
                Write-Host "`r   $($spinner[$i % $spinner.Length]) Downloading... " -NoNewline -ForegroundColor Yellow
                Start-Sleep -Milliseconds 100
                $i++
            }

            $result = Receive-Job -Job $job
            Remove-Job -Job $job
            Write-Host "`r                                                    " -NoNewline

            if ($LASTEXITCODE -eq 0) {
                Write-Host "   ✅ Target $target installed successfully" -ForegroundColor Green
            } else {
                Write-Host "   ❌ Installation of $target failed" -ForegroundColor Red
                Write-Host $result -ForegroundColor Red
                return $false
            }
        } else {
            Write-Host "   ✅ Target $target already installed" -ForegroundColor Green
        }
    }

    return $true
}

function Create-ReleaseFolder {
    $releasePath = "release"
    if (Test-Path $releasePath) {
        Remove-Item -Path $releasePath -Recurse -Force
    }
    New-Item -Path $releasePath -ItemType Directory | Out-Null
    Write-Host "📁 Dossier release créé" -ForegroundColor Green
}

function Build-UI {
    Write-Host "`n🎨 Building UI..." -ForegroundColor Cyan
    
    $targets = Show-TargetMenu
    if ($null -eq $targets) {
        return
    }

    $progress = 0
    $totalTargets = $targets.Count

    foreach ($target in $targets) {
        $progress++
        $percent = [math]::Round(($progress / $totalTargets) * 100)
        Write-Progress -Activity "Building UI" -Status "Target: $target" -PercentComplete $percent

        Write-Host "   🔨 Building for $target..." -ForegroundColor Yellow
        $buildResult = cargo build --release --target $target -p hot-reload-ui
        if ($LASTEXITCODE -ne 0) {
            Write-Host "   ❌ Échec du build pour $target" -ForegroundColor Red
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

function Build-Watcher {
    Write-Host "`n👀 Building Watcher..." -ForegroundColor Cyan
    
    $targets = @("x86_64-pc-windows-msvc", "x86_64-unknown-linux-gnu")
    $availableTargets = $targets | Where-Object {
        $installed = rustup target list | Select-String "^$_" | Select-String "installed"
        return $installed
    }

    if ($availableTargets.Count -eq 0) {
        Write-Host "   ❌ Aucune target compatible installée" -ForegroundColor Red
        return
    }

    $progress = 0
    $totalTargets = $availableTargets.Count

    foreach ($target in $availableTargets) {
        $progress++
        $percent = [math]::Round(($progress / $totalTargets) * 100)
        Write-Progress -Activity "Building Watcher" -Status "Target: $target" -PercentComplete $percent

        Write-Host "   🔨 Building for $target..." -ForegroundColor Yellow
        $buildResult = cargo build --release --target $target -p hot-reload-watcher
        if ($LASTEXITCODE -ne 0) {
            Write-Host "   ❌ Échec du build pour $target" -ForegroundColor Red
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
        } else {
            Write-Host "   ❌ Binary not found: $sourcePath" -ForegroundColor Red
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
        Write-Host "   ❌ Échec du build FX Server" -ForegroundColor Red
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
Install-RustTargets
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