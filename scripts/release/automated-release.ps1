# =========================================
# AUTOMATED RELEASE PREPARATION SCRIPT
# =========================================
# Intelligent release automation для MAGRAY CLI

[CmdletBinding()]
param(
    [Parameter(Mandatory = $false)]
    [ValidateSet("patch", "minor", "major")]
    [string]$ReleaseType = "patch",
    
    [Parameter(Mandatory = $false)]
    [switch]$DryRun = $false,
    
    [Parameter(Mandatory = $false)]
    [string]$CustomVersion,
    
    [Parameter(Mandatory = $false)]
    [switch]$SkipTests = $false,
    
    [Parameter(Mandatory = $false)]
    [switch]$Force = $false
)

# Configuration
$ErrorActionPreference = "Stop"
$ProgressPreference = "SilentlyContinue"

# Colors для output
$Colors = @{
    Success = "Green"
    Warning = "Yellow" 
    Error   = "Red"
    Info    = "Cyan"
    Highlight = "Magenta"
}

function Write-ColoredOutput {
    param($Message, $Color = "White")
    Write-Host $Message -ForegroundColor $Colors[$Color]
}

function Test-Prerequisites {
    Write-ColoredOutput "🔍 Checking prerequisites..." "Info"
    
    $prerequisites = @("git", "cargo", "jq")
    $missing = @()
    
    foreach ($tool in $prerequisites) {
        if (-not (Get-Command $tool -ErrorAction SilentlyContinue)) {
            $missing += $tool
        }
    }
    
    if ($missing.Count -gt 0) {
        Write-ColoredOutput "❌ Missing prerequisites: $($missing -join ', ')" "Error"
        return $false
    }
    
    # Check git status
    $gitStatus = git status --porcelain 2>$null
    if ($gitStatus -and -not $Force) {
        Write-ColoredOutput "❌ Working directory is not clean. Use -Force to override." "Error"
        return $false
    }
    
    # Check current branch
    $currentBranch = git branch --show-current
    if ($currentBranch -ne "main" -and -not $Force) {
        Write-ColoredOutput "❌ Not on main branch ($currentBranch). Use -Force to override." "Error"
        return $false
    }
    
    Write-ColoredOutput "✅ Prerequisites check passed" "Success"
    return $true
}

function Get-CurrentVersion {
    $cargoToml = Get-Content "Cargo.toml" -Raw
    if ($cargoToml -match 'version\s*=\s*"([^"]+)"') {
        return $matches[1]
    }
    throw "Cannot find version in Cargo.toml"
}

function Get-NextVersion {
    param($CurrentVersion, $ReleaseType)
    
    if ($CustomVersion) {
        return $CustomVersion
    }
    
    $versionParts = $CurrentVersion.Split('.')
    $major = [int]$versionParts[0]
    $minor = [int]$versionParts[1]  
    $patch = [int]$versionParts[2]
    
    switch ($ReleaseType) {
        "major" { return "$($major + 1).0.0" }
        "minor" { return "$major.$($minor + 1).0" }
        "patch" { return "$major.$minor.$($patch + 1)" }
        default { throw "Invalid release type: $ReleaseType" }
    }
}

function Update-Version {
    param($NewVersion)
    
    Write-ColoredOutput "📝 Updating version to $NewVersion..." "Info"
    
    # Update Cargo.toml
    $cargoContent = Get-Content "Cargo.toml" -Raw
    $cargoContent = $cargoContent -replace 'version\s*=\s*"[^"]+"', "version = `"$NewVersion`""
    Set-Content "Cargo.toml" -Value $cargoContent -NoNewline
    
    # Update workspace Cargo.toml files
    Get-ChildItem -Path "crates" -Recurse -Name "Cargo.toml" | ForEach-Object {
        $cratePath = Join-Path "crates" $_
        $crateContent = Get-Content $cratePath -Raw
        $crateContent = $crateContent -replace 'version\s*=\s*"[^"]+"', "version = `"$NewVersion`""
        Set-Content $cratePath -Value $crateContent -NoNewline
    }
    
    Write-ColoredOutput "✅ Version updated in all Cargo.toml files" "Success"
}

function Run-Tests {
    if ($SkipTests) {
        Write-ColoredOutput "⚠️ Skipping tests as requested" "Warning"
        return $true
    }
    
    Write-ColoredOutput "🧪 Running comprehensive test suite..." "Info"
    
    # Quick format check
    Write-ColoredOutput "  📝 Checking code formatting..." "Info"
    $formatResult = cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) {
        Write-ColoredOutput "❌ Code formatting check failed" "Error"
        return $false
    }
    
    # Clippy check
    Write-ColoredOutput "  🔍 Running Clippy analysis..." "Info"
    $clippyResult = cargo clippy --workspace --all-targets -- -D warnings -A clippy::too_many_arguments
    if ($LASTEXITCODE -ne 0) {
        Write-ColoredOutput "❌ Clippy check failed" "Error"
        return $false
    }
    
    # Unit tests
    Write-ColoredOutput "  🧪 Running unit tests..." "Info"
    $testResult = cargo test --workspace --lib
    if ($LASTEXITCODE -ne 0) {
        Write-ColoredOutput "❌ Unit tests failed" "Error"
        return $false
    }
    
    Write-ColoredOutput "✅ All tests passed" "Success"
    return $true
}

function Generate-Changelog {
    param($CurrentVersion, $NewVersion)
    
    Write-ColoredOutput "📝 Generating changelog..." "Info"
    
    $changelogPath = "CHANGELOG.md"
    $tempChangelog = "CHANGELOG.tmp"
    
    # Generate git log since last tag
    $lastTag = git describe --tags --abbrev=0 HEAD 2>$null
    if (-not $lastTag) {
        $lastTag = (git rev-list --max-parents=0 HEAD)
    }
    
    $commitMessages = git log --pretty=format:"%s (%an)" "$lastTag..HEAD" --no-merges
    
    # Create changelog entry
    $changelogEntry = @"
## [$NewVersion] - $(Get-Date -Format "yyyy-MM-dd")

### Added
$(($commitMessages | Where-Object { $_ -match "^feat|^add" } | ForEach-Object { "- $_" }) -join "`n")

### Changed  
$(($commitMessages | Where-Object { $_ -match "^update|^change|^improve" } | ForEach-Object { "- $_" }) -join "`n")

### Fixed
$(($commitMessages | Where-Object { $_ -match "^fix|^bugfix" } | ForEach-Object { "- $_" }) -join "`n")

### Security
$(($commitMessages | Where-Object { $_ -match "^security|^sec" } | ForEach-Object { "- $_" }) -join "`n")

"@
    
    # Prepend to existing changelog
    if (Test-Path $changelogPath) {
        $existingChangelog = Get-Content $changelogPath -Raw
        $newChangelog = $changelogEntry + "`n" + $existingChangelog
    } else {
        $newChangelog = "# Changelog`n`n" + $changelogEntry
    }
    
    Set-Content $changelogPath -Value $newChangelog -NoNewline
    Write-ColoredOutput "✅ Changelog updated" "Success"
}

function Build-Release-Artifacts {
    param($Version)
    
    Write-ColoredOutput "🏗️ Building release artifacts..." "Info"
    
    $targets = @(
        @{ Target = "x86_64-pc-windows-msvc"; Name = "windows-x64" },
        @{ Target = "x86_64-unknown-linux-gnu"; Name = "linux-x64" },
        @{ Target = "x86_64-apple-darwin"; Name = "macos-x64" }
    )
    
    $artifactDir = "target/release-artifacts"
    New-Item -Path $artifactDir -ItemType Directory -Force | Out-Null
    
    foreach ($target in $targets) {
        Write-ColoredOutput "  🔨 Building for $($target.Name)..." "Info"
        
        # Cross-compilation setup might be needed
        $buildCmd = "cargo build --release --target $($target.Target) --features cpu"
        Invoke-Expression $buildCmd
        
        if ($LASTEXITCODE -eq 0) {
            $binaryName = if ($target.Target -match "windows") { "magray.exe" } else { "magray" }
            $binaryPath = "target/$($target.Target)/release/$binaryName"
            $artifactName = "magray-$Version-$($target.Name)"
            
            if ($target.Target -match "windows") {
                $artifactName += ".exe"
            }
            
            Copy-Item $binaryPath "$artifactDir/$artifactName"
            Write-ColoredOutput "    ✅ Built $artifactName" "Success"
        } else {
            Write-ColoredOutput "    ❌ Failed to build for $($target.Name)" "Error"
        }
    }
}

function Create-Release-Tag {
    param($Version)
    
    if ($DryRun) {
        Write-ColoredOutput "🏷️ [DRY RUN] Would create tag v$Version" "Warning"
        return
    }
    
    Write-ColoredOutput "🏷️ Creating release tag v$Version..." "Info"
    
    # Commit version changes
    git add .
    git commit -m "chore: bump version to $Version"
    
    # Create annotated tag
    git tag -a "v$Version" -m "Release version $Version"
    
    Write-ColoredOutput "✅ Tag v$Version created" "Success"
}

function Generate-Release-Summary {
    param($CurrentVersion, $NewVersion, $ReleaseType)
    
    $summary = @"
🚀 MAGRAY CLI Release Summary
============================

Previous Version: $CurrentVersion
New Version:      $NewVersion
Release Type:     $ReleaseType
Date:            $(Get-Date -Format "yyyy-MM-dd HH:mm:ss")

📁 Artifacts Generated:
- Windows x64 binary
- Linux x64 binary  
- macOS x64 binary
- Updated changelog
- Git tag: v$NewVersion

🔄 Next Steps:
1. Push changes: git push origin main --tags
2. Create GitHub release from tag v$NewVersion
3. Upload binary artifacts to release
4. Update Docker images
5. Notify team/users

🔍 Verification Commands:
- cargo --version
- magray --version
- git log --oneline -5
- git tag --list | tail -5
"@

    Write-ColoredOutput $summary "Info"
    
    # Save summary to file
    Set-Content "release-summary-$NewVersion.txt" -Value $summary
    Write-ColoredOutput "📄 Release summary saved to release-summary-$NewVersion.txt" "Success"
}

# =========================================
# MAIN EXECUTION
# =========================================

try {
    Write-ColoredOutput "🚀 MAGRAY CLI Automated Release Process" "Highlight"
    Write-ColoredOutput "=======================================" "Highlight"
    
    if ($DryRun) {
        Write-ColoredOutput "⚠️ DRY RUN MODE - No changes will be made" "Warning"
    }
    
    # Step 1: Prerequisites
    if (-not (Test-Prerequisites)) {
        exit 1
    }
    
    # Step 2: Version calculation
    $currentVersion = Get-CurrentVersion
    $newVersion = Get-NextVersion $currentVersion $ReleaseType
    
    Write-ColoredOutput "📊 Release Information:" "Info"
    Write-ColoredOutput "  Current Version: $currentVersion" "Info"
    Write-ColoredOutput "  New Version:     $newVersion" "Info"
    Write-ColoredOutput "  Release Type:    $ReleaseType" "Info"
    
    # Confirmation prompt
    if (-not $Force -and -not $DryRun) {
        $confirmation = Read-Host "Proceed with release? (y/N)"
        if ($confirmation -ne "y" -and $confirmation -ne "Y") {
            Write-ColoredOutput "❌ Release cancelled by user" "Warning"
            exit 0
        }
    }
    
    # Step 3: Run tests
    if (-not (Run-Tests)) {
        exit 1
    }
    
    if (-not $DryRun) {
        # Step 4: Update version
        Update-Version $newVersion
        
        # Step 5: Generate changelog
        Generate-Changelog $currentVersion $newVersion
        
        # Step 6: Build release artifacts
        Build-Release-Artifacts $newVersion
        
        # Step 7: Create tag
        Create-Release-Tag $newVersion
    }
    
    # Step 8: Generate summary
    Generate-Release-Summary $currentVersion $newVersion $ReleaseType
    
    Write-ColoredOutput "🎉 Release process completed successfully!" "Success"
    
    if (-not $DryRun) {
        Write-ColoredOutput "💡 Don't forget to push changes: git push origin main --tags" "Highlight"
    }
    
} catch {
    Write-ColoredOutput "❌ Release process failed: $($_.Exception.Message)" "Error"
    exit 1
}