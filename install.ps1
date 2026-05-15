# j-cli Windows 安装脚本
# 使用方式:
#   irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex
#   或者指定版本: $v="v1.0.0"; irm https://raw.githubusercontent.com/LingoJack/jcli/main/install.ps1 | iex
#
# 也可直接下载后执行:
#   powershell -ExecutionPolicy Bypass -File install.ps1
#   powershell -ExecutionPolicy Bypass -File install.ps1 -Version v1.0.0
#   powershell -ExecutionPolicy Bypass -File install.ps1 -Uninstall

param(
    [string]$Version = "",
    [switch]$Uninstall,
    [switch]$Help
)

$ErrorActionPreference = "Stop"

# 配置
$Repo = "LingoJack/jcli"
$BinaryName = "j"
$InstallDir = "$env:LOCALAPPDATA\j-cli"
$DataDir = "$env:USERPROFILE\.jdata"


function Write-Info($msg) {
    Write-Host "[INFO] $msg" -ForegroundColor Green
}

function Write-Warn($msg) {
    Write-Host "[WARN] $msg" -ForegroundColor Yellow
}

function Write-Err($msg) {
    Write-Host "[ERROR] $msg" -ForegroundColor Red
}

# 检测平台
function Detect-Platform {
    $arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture
    if ($null -eq $arch) {
        # .NET Framework fallback
        $arch = if ([Environment]::Is64BitOperatingSystem) { "X64" } else { "X86" }
    }

    switch ($arch.ToString()) {
        { $_ -match "X64|AMD64" } { return "windows-x64" }
        { $_ -match "ARM64" } { return "windows-arm64" }
        { $_ -match "X86" } {
            Write-Warn "32 位 Windows 不受官方支持，尝试使用 x64 版本"
            return "windows-x64"
        }
        default { Write-Err "不支持的架构: $arch"; return }
    }
}

# 获取最新版本号
function Get-LatestVersion {
    $apiUrl = "https://api.github.com/repos/$Repo/releases/latest"

    # 方法1: GitHub API
    try {
        Write-Info "正在从 GitHub API 获取最新版本..."
        $response = Invoke-RestMethod -Uri $apiUrl -Headers @{ "User-Agent" = "j-cli-installer" } -TimeoutSec 15
        $tag = $response.tag_name
        if ($tag -match '^v\d+\.\d+\.\d+$') {
            return $tag
        }
    }
    catch {
        Write-Warn "GitHub API 访问失败: $($_.Exception.Message)"
    }

    # 方法2: 跟随 releases/latest 重定向，直接从最终 URL 提取 tag
    try {
        Write-Info "正在从 releases 页面获取版本..."
        $response = Invoke-WebRequest -Uri "https://github.com/$Repo/releases/latest" -Headers @{ "User-Agent" = "j-cli-installer" } -TimeoutSec 15 -UseBasicParsing
        $finalUrl = $response.BaseResponse.ResponseUri.AbsoluteUri
        if ($finalUrl -match '(v\d+\.\d+\.\d+)$') {
            return $Matches[1]
        }
    }
    catch {
        Write-Warn "Releases 页面访问失败: $($_.Exception.Message)"
    }

    # 所有网络方法均失败，直接报错
    Write-Err "无法从网络获取最新版本，请检查网络连接后重试。如需安装指定版本，请使用: install.ps1 -Version v1.0.0"
    return
}

# 下载并安装
function Install-JCli {
    param([string]$Version, [string]$Platform)

    if ([string]::IsNullOrEmpty($Version)) {
        Write-Info "正在获取最新版本..."
        $Version = Get-LatestVersion
    }

    Write-Info "安装版本: $Version"
    Write-Info "平台: $Platform"

    # 构建下载 URL
    $assetName = "j-$Platform"
    $downloadUrl = "https://github.com/$Repo/releases/download/$Version/$assetName.zip"
    Write-Info "下载地址: $downloadUrl"

    # 创建临时目录
    $tmpDir = Join-Path $env:TEMP "j-install-$(Get-Random)"
    New-Item -ItemType Directory -Path $tmpDir -Force | Out-Null

    try {
        # 下载
        Write-Info "正在下载..."
        $zipFile = Join-Path $tmpDir "j.zip"

        try {
            Invoke-WebRequest -Uri $downloadUrl -OutFile $zipFile -Headers @{ "User-Agent" = "j-cli-installer" } -UseBasicParsing
        }
        catch {
            Write-Err "下载失败，请检查版本号是否正确或网络连接是否正常`n$($_.Exception.Message)"
            return
        }

        # 解压
        Write-Info "正在解压..."
        Expand-Archive -Path $zipFile -DestinationPath $tmpDir -Force

        # 创建安装目录
        if (-not (Test-Path $InstallDir)) {
            New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
            Write-Info "已创建安装目录: $InstallDir"
        }

        # 安装
        $srcExe = Join-Path $tmpDir "$BinaryName.exe"
        if (-not (Test-Path $srcExe)) {
            # 尝试从子目录查找
            $srcExe = Get-ChildItem -Path $tmpDir -Filter "$BinaryName.exe" -Recurse | Select-Object -First 1
            if ($null -eq $srcExe) {
                Write-Err "解压后未找到 $BinaryName.exe"
                return
            }
            $srcExe = $srcExe.FullName
        }

        $dstExe = Join-Path $InstallDir "$BinaryName.exe"

        # 如果目标 j.exe 正在运行，先尝试关闭占用进程
        if (Test-Path $dstExe) {
            try {
                # 尝试复制，如果成功则无需特殊处理
                Copy-Item -Path $srcExe -Destination $dstExe -Force -ErrorAction Stop
            }
            catch {
                # 复制失败（文件被占用），先关闭 j.exe 进程
                Write-Warn "检测到 $BinaryName.exe 正在运行，正在尝试关闭..."
                $procs = Get-Process -Name $BinaryName -ErrorAction SilentlyContinue
                if ($procs) {
                    $procs | Stop-Process -Force
                    Start-Sleep -Milliseconds 500
                    Write-Info "已关闭旧进程"
                }

                # 关闭进程后重试复制
                try {
                    Copy-Item -Path $srcExe -Destination $dstExe -Force -ErrorAction Stop
                }
                catch {
                    # 如果仍然失败，使用重命名策略：先重命名旧文件，再复制新文件
                    Write-Warn "文件仍被占用，使用重命名策略..."
                    $backupExe = "$dstExe.bak"
                    if (Test-Path $backupExe) { Remove-Item -Path $backupExe -Force -ErrorAction SilentlyContinue }
                    Rename-Item -Path $dstExe -NewName "$BinaryName.exe.bak" -Force
                    Copy-Item -Path $srcExe -Destination $dstExe -Force
                    Write-Info "旧版本已备份为 $BinaryName.exe.bak"
                    # 延迟清理备份文件（当前进程退出后）
                    $cleanupScript = "Start-Sleep -Seconds 3; Remove-Item '$backupExe' -Force -ErrorAction SilentlyContinue"
                    Start-Process -FilePath "powershell.exe" -ArgumentList "-NoProfile", "-WindowStyle", "Hidden", "-Command", $cleanupScript -WindowStyle Hidden
                }
            }
        }
        else {
            Copy-Item -Path $srcExe -Destination $dstExe -Force
        }

        Write-Info "已安装到: $InstallDir\$BinaryName.exe"

        # 添加到 PATH（用户级）
        $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
        if ($userPath -notlike "*$InstallDir*") {
            [Environment]::SetEnvironmentVariable("Path", "$userPath;$InstallDir", "User")
            # 更新当前会话 PATH
            $env:Path = "$env:Path;$InstallDir"
            Write-Info "已将 $InstallDir 添加到用户 PATH"
        }
        else {
            Write-Info "$InstallDir 已在 PATH 中"
        }

        # 验证安装
        $installedExe = Join-Path $InstallDir "$BinaryName.exe"
        if (Test-Path $installedExe) {
            Write-Info ""
            Write-Info "====================================="
            Write-Info "  安装成功!"
            Write-Info "====================================="
            Write-Info ""
            Write-Info "安装位置: $InstallDir\$BinaryName.exe"
            Write-Info "数据目录: $DataDir"
            Write-Info ""
            Write-Info "请重新打开终端窗口以使 PATH 生效"
            Write-Info "然后运行 'j version' 查看版本信息"
            Write-Info "运行 'j help' 查看帮助文档"
        }
        else {
            Write-Err "安装失败"
            return
        }
    }
    finally {
        # 清理临时目录
        if (Test-Path $tmpDir) {
            Remove-Item -Path $tmpDir -Recurse -Force -ErrorAction SilentlyContinue
        }
    }
}

# 卸载
function Uninstall-JCli {
    Write-Info "正在卸载..."

    $exePath = Join-Path $InstallDir "$BinaryName.exe"
    if (Test-Path $exePath) {
        # 先尝试关闭 j 进程
        $procs = Get-Process -Name $BinaryName -ErrorAction SilentlyContinue
        if ($procs) {
            Write-Warn "检测到 $BinaryName.exe 正在运行，正在关闭..."
            $procs | Stop-Process -Force
            Start-Sleep -Milliseconds 500
        }

        try {
            Remove-Item -Path $exePath -Force -ErrorAction Stop
            Write-Info "已删除 $exePath"
        }
        catch {
            # 文件仍被占用，使用重命名策略
            Write-Warn "文件被占用，使用重命名策略..."
            Rename-Item -Path $exePath -NewName "$BinaryName.exe.bak" -Force
            Write-Info "已重命名 $exePath 为 $BinaryName.exe.bak"
            # 延迟清理
            $cleanupScript = "Start-Sleep -Seconds 3; Remove-Item '$exePath.bak' -Force -ErrorAction SilentlyContinue"
            Start-Process -FilePath "powershell.exe" -ArgumentList "-NoProfile", "-WindowStyle", "Hidden", "-Command", $cleanupScript -WindowStyle Hidden
        }
    }
    else {
        Write-Warn "程序未安装在 $InstallDir"
    }

    # 清理空目录
    if ((Test-Path $InstallDir) -and ((Get-ChildItem $InstallDir -ErrorAction SilentlyContinue).Count -eq 0)) {
        Remove-Item -Path $InstallDir -Force
        Write-Info "已删除安装目录 $InstallDir"
    }

    # 从 PATH 移除
    $userPath = [Environment]::GetEnvironmentVariable("Path", "User")
    if ($userPath -like "*$InstallDir*") {
        $newPath = ($userPath -split ";" | Where-Object { $_ -ne $InstallDir -and $_ -ne "" }) -join ";"
        [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
        Write-Info "已从 PATH 移除 $InstallDir"
    }

    Write-Info ""
    Write-Warn "数据目录 $DataDir 未删除，如需彻底清理请手动删除:"
    Write-Host "  Remove-Item -Recurse -Force `"$DataDir`"" -ForegroundColor Cyan
}

# 显示帮助
function Show-Help {
    Write-Host "j-cli Windows 安装脚本"
    Write-Host ""
    Write-Host "使用方式:"
    Write-Host "  irm https://raw.githubusercontent.com/$Repo/main/install.ps1 | iex"
    Write-Host ""
    Write-Host "指定版本安装:"
    Write-Host '  $v="v1.0.0"; irm https://raw.githubusercontent.com/$Repo/main/install.ps1 | iex'
    Write-Host ""
    Write-Host "直接执行脚本:"
    Write-Host "  powershell -ExecutionPolicy Bypass -File install.ps1"
    Write-Host "  powershell -ExecutionPolicy Bypass -File install.ps1 -Version v1.0.0"
    Write-Host ""
    Write-Host "卸载:"
    Write-Host "  powershell -ExecutionPolicy Bypass -File install.ps1 -Uninstall"
    Write-Host ""
    Write-Host "参数:"
    Write-Host "  -Version      指定安装版本 (如 v1.0.0)"
    Write-Host "  -Uninstall    卸载程序"
    Write-Host "  -Help         显示帮助信息"
}

# 主入口
function Main {
    if ($Help) {
        Show-Help
        return
    }

    Write-Host ""
    Write-Host "=======================================" -ForegroundColor Cyan
    Write-Host "  j-cli Windows 安装程序" -ForegroundColor Cyan
    Write-Host "  快捷命令行工具 - Rust 实现" -ForegroundColor Cyan
    Write-Host "=======================================" -ForegroundColor Cyan
    Write-Host ""

    if ($Uninstall) {
        Uninstall-JCli
    }
    else {
        $platform = Detect-Platform
        if ([string]::IsNullOrEmpty($platform)) {
            Write-Err "无法检测平台，安装终止"
            return
        }
        Install-JCli -Version $Version -Platform $platform
    }
}

Main
