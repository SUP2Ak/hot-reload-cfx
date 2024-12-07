function Show-TargetMenu {
    Write-Host "`n📦 Targets installation selection" -ForegroundColor Cyan
    Write-Host "------------------------"
    Write-Host "1. Windows (x86_64-pc-windows-msvc)"
    Write-Host "2. Linux (x86_64-unknown-linux-musl)"
    Write-Host "3. MacOS (x86_64-apple-darwin)"
    Write-Host "4. All targets"
    Write-Host "5. Exit"
    Write-Host "------------------------"

    $choice = Read-Host "Choose an option"
    
    $targets = @()
    switch ($choice) {
        "1" { $targets = @("x86_64-pc-windows-msvc") }
        "2" { $targets = @("x86_64-unknown-linux-musl") }
        "3" { $targets = @("x86_64-apple-darwin") }
        "4" { $targets = @("x86_64-pc-windows-msvc", "x86_64-unknown-linux-musl", "x86_64-apple-darwin") }
        "5" { return $null }
        default { 
            Write-Host "❌ Invalid option" -ForegroundColor Red
            return $null
        }
    }
    return $targets
}

function Install-CrossTools {
    Write-Host "📥 Installing cross-rs..." -ForegroundColor Yellow
    
    # Installer Docker Desktop si nécessaire
    if (-not (Get-Command "docker" -ErrorAction SilentlyContinue)) {
        Write-Host "   Installing Docker Desktop..." -ForegroundColor Yellow
        winget install Docker.DockerDesktop
        Write-Host "⚠️ Please restart your computer after Docker installation" -ForegroundColor Yellow
        return $false
    }
    
    # Installer cross
    cargo install cross
    
    Write-Host "✅ cross-rs installed successfully" -ForegroundColor Green
    return $true
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
            
            if ($target -eq "x86_64-unknown-linux-musl") {
                Install-CrossTools
            }
            
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

# Main loop
do {
    Clear-Host
    Write-Host "🎯 Rust Target Installer" -ForegroundColor Magenta
    Write-Host "------------------------"
    
    Install-RustTargets
    
    Write-Host "`nPress any key to continue or 'Q' to quit..."
    $key = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
    if ($key.Character -eq 'q' -or $key.Character -eq 'Q') {
        Write-Host "`n👋 Goodbye!" -ForegroundColor Cyan
        break
    }
} while ($true) 
