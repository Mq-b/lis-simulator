# LIS 模拟器 集成测试脚本
# 使用 com0com 虚拟串口对进行端到端测试

param(
    [string]$ComPort1 = "COM10",
    [string]$ComPort2 = "COM11",
    [int]$BaudRate = 9600
)

$ErrorActionPreference = "Stop"

Write-Host "========================================" -ForegroundColor Cyan
Write-Host "  LIS 模拟器 集成测试" -ForegroundColor Cyan
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""

# ─── 检查 com0com ────────────────────────────────
Write-Host "[1/5] 检查 com0com..." -ForegroundColor Yellow
$com0comPath = $null
$paths = @(
    "C:\Program Files\com0com",
    "C:\Program Files (x86)\com0com",
    "C:\Program Files (x86)\Bojan Strbac\com0com"
)
foreach ($p in $paths) {
    if (Test-Path "$p\setupc.exe") {
        $com0comPath = "$p\setupc.exe"
        break
    }
}

if (-not $com0comPath) {
    Write-Host "  [FAIL] 未找到 com0com，请安装:" -ForegroundColor Red
    Write-Host "  下载地址: https://sourceforge.net/projects/com0com/" -ForegroundColor White
    Write-Host "  安装后重新运行此脚本" -ForegroundColor White
    exit 1
}
Write-Host "  [OK] com0com 已安装: $com0comPath" -ForegroundColor Green

# ─── 检查虚拟串口是否存在 ────────────────────────
Write-Host ""
Write-Host "[2/5] 检查虚拟串口 $ComPort1 <-> $ComPort2..." -ForegroundColor Yellow

$ports = [System.IO.Ports.SerialPort]::GetPortNames()
if ($ComPort1 -notin $ports -or $ComPort2 -notin $ports) {
    Write-Host "  虚拟串口不存在，尝试创建..." -ForegroundColor Yellow
    try {
        & $com0comPath install PortName=$ComPort1 PortName=$ComPort2 2>&1 | Out-Null
        Start-Sleep -Seconds 2
        Write-Host "  [OK] 虚拟串口已创建" -ForegroundColor Green
    } catch {
        Write-Host "  [FAIL] 无法创建虚拟串口，请以管理员身份运行" -ForegroundColor Red
        Write-Host "  手动创建: 打开 com0com Setup，添加 CNCA0=$ComPort1 CNCB0=$ComPort2" -ForegroundColor White
        exit 1
    }
} else {
    Write-Host "  [OK] 虚拟串口已存在" -ForegroundColor Green
}

# ─── 检查 pyserial ───────────────────────────────
Write-Host ""
Write-Host "[3/5] 检查 Python 环境..." -ForegroundColor Yellow
$pyOk = python -c "import serial; print('ok')" 2>&1
if ($pyOk -ne "ok") {
    Write-Host "  [FAIL] pyserial 未安装，正在安装..." -ForegroundColor Red
    pip install pyserial
}
Write-Host "  [OK] pyserial 已就绪" -ForegroundColor Green

# ─── 检查项目编译 ───────────────────────────────
Write-Host ""
Write-Host "[4/5] 检查项目编译..." -ForegroundColor Yellow
$buildResult = cargo build --manifest-path "E:\project\LIS模拟器\Cargo.toml" 2>&1
if ($LASTEXITCODE -ne 0) {
    Write-Host "  [FAIL] 编译失败" -ForegroundColor Red
    Write-Host $buildResult
    exit 1
}
Write-Host "  [OK] 编译成功" -ForegroundColor Green

# ─── 运行集成测试 ───────────────────────────────
Write-Host ""
Write-Host "[5/5] 运行集成测试..." -ForegroundColor Yellow
Write-Host ""

# 测试 1: 启动 LIS 模拟器
Write-Host "--- 测试 1: 软件启动 ---" -ForegroundColor Cyan
$lisProcess = Start-Process -FilePath "E:\project\LIS模拟器\target\debug\lis-simulator.exe" `
    -PassThru -WindowStyle Normal
Start-Sleep -Seconds 2

if ($lisProcess.HasExited) {
    Write-Host "  [FAIL] LIS 模拟器启动后立即退出" -ForegroundColor Red
    exit 1
}
Write-Host "  [OK] LIS 模拟器已启动 (PID: $($lisProcess.Id))" -ForegroundColor Green

# 测试 2: 仪器模拟器发送数据
Write-Host ""
Write-Host "--- 测试 2: 仪器发送 ASTM 数据 ---" -ForegroundColor Cyan
Write-Host "  请在 LIS 模拟器中:" -ForegroundColor White
Write-Host "  1. 选择 $ComPort1" -ForegroundColor White
Write-Host "  2. 波特率选择 $BaudRate" -ForegroundColor White
Write-Host "  3. 点击'开始监听'" -ForegroundColor White
Write-Host ""
Write-Host "  然后按 Enter 继续测试..." -ForegroundColor White
Read-Host

# 运行仪器模拟器
$simResult = python "E:\project\LIS模拟器\tests\instrument_simulator.py" `
    --port $ComPort2 --baud $BaudRate 2>&1

Write-Host ""
Write-Host "  仪器模拟器输出:" -ForegroundColor White
Write-Host $simResult

# 验证
Write-Host ""
Write-Host "--- 验证结果 ---" -ForegroundColor Cyan
Write-Host "请检查 LIS 模拟器界面:" -ForegroundColor White
Write-Host "  [ ] 原始数据日志显示了 ENQ/ACK/DATA/EOT 记录" -ForegroundColor White
Write-Host "  [ ] 解析结果显示了消息 #1 (患者: 张三)" -ForegroundColor White
Write-Host "  [ ] 检验结果表格显示了 cTnI/CK-MB/NT-proBNP" -ForegroundColor White
Write-Host "  [ ] 状态栏显示消息数和结果数" -ForegroundColor White
Write-Host ""
Write-Host "是否全部通过? (Y/N)" -ForegroundColor Yellow
$pass = Read-Host

# 清理
Write-Host ""
Write-Host "清理: 关闭 LIS 模拟器..." -ForegroundColor Yellow
if (-not $lisProcess.HasExited) {
    Stop-Process -Id $lisProcess.Id -Force -ErrorAction SilentlyContinue
}

if ($pass -eq "Y" -or $pass -eq "y") {
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Green
    Write-Host "  集成测试通过!" -ForegroundColor Green
    Write-Host "========================================" -ForegroundColor Green
} else {
    Write-Host ""
    Write-Host "========================================" -ForegroundColor Red
    Write-Host "  集成测试未通过，请检查日志" -ForegroundColor Red
    Write-Host "========================================" -ForegroundColor Red
    exit 1
}
