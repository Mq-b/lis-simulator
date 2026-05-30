#!/bin/bash
# 虚拟串口管理脚本
# 使用 socat 创建虚拟串口对，用于 LIS 模拟器串口调试
#
# 用法:
#   sudo ./tests/setup_vserial.sh start    # 创建虚拟串口
#   sudo ./tests/setup_vserial.sh stop     # 销毁虚拟串口
#   sudo ./tests/setup_vserial.sh status   # 查看状态

set -e

# 配置 - 符号链接必须在 /dev 目录下，serialport 库才能识别
SERIAL_PORT_LIS="/dev/ttyV0"
SERIAL_PORT_INSTRUMENT="/dev/ttyV1"
PIDFILE="/tmp/vserial_socat.pid"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# 检查 root 权限
check_root() {
    if [ "$(id -u)" -ne 0 ]; then
        echo -e "${RED}错误: 需要 root 权限${NC}"
        echo "请使用: sudo $0 $1"
        exit 1
    fi
}

# 检查 socat 是否安装
check_socat() {
    if ! command -v socat &> /dev/null; then
        echo -e "${RED}错误: socat 未安装${NC}"
        echo "请运行: sudo apt install socat"
        exit 1
    fi
}

# 创建虚拟串口
start_vserial() {
    check_root "start"
    check_socat

    # 检查是否已存在
    if [ -f "$PIDFILE" ]; then
        local pid=$(cat "$PIDFILE")
        if kill -0 "$pid" 2>/dev/null; then
            echo -e "${YELLOW}虚拟串口已存在 (PID: $pid)${NC}"
            echo "端口: $SERIAL_PORT_LIS <-> $SERIAL_PORT_INSTRUMENT"
            return 0
        else
            rm -f "$PIDFILE"
        fi
    fi

    # 清理旧的符号链接
    rm -f "$SERIAL_PORT_LIS" "$SERIAL_PORT_INSTRUMENT"

    echo "正在创建虚拟串口..."

    # 使用 socat 创建虚拟串口对
    # mode=666 让 PTY 设备权限为所有人可读写
    # link= 创建符号链接到 /dev/ 目录
    socat \
        PTY,raw,echo=0,mode=666,link="$SERIAL_PORT_LIS" \
        PTY,raw,echo=0,mode=666,link="$SERIAL_PORT_INSTRUMENT" &
    local pid=$!

    # 等待 socat 启动
    sleep 0.5

    # 检查进程是否存活
    if ! kill -0 "$pid" 2>/dev/null; then
        echo -e "${RED}错误: socat 启动失败${NC}"
        exit 1
    fi

    # 保存 PID
    echo "$pid" > "$PIDFILE"

    # 等待符号链接创建
    local retries=0
    while [ ! -L "$SERIAL_PORT_LIS" ] || [ ! -L "$SERIAL_PORT_INSTRUMENT" ]; do
        sleep 0.1
        retries=$((retries + 1))
        if [ $retries -gt 50 ]; then
            echo -e "${RED}错误: 虚拟串口创建超时${NC}"
            kill "$pid" 2>/dev/null
            rm -f "$PIDFILE"
            exit 1
        fi
    done

    # 获取实际 PTY 设备路径
    local real_lis=$(readlink -f "$SERIAL_PORT_LIS")
    local real_instr=$(readlink -f "$SERIAL_PORT_INSTRUMENT")

    echo -e "${GREEN}虚拟串口创建成功${NC}"
    echo "  LIS 端口:       $SERIAL_PORT_LIS -> $real_lis"
    echo "  仪器端口:       $SERIAL_PORT_INSTRUMENT -> $real_instr"
    echo "  socat PID:      $pid"
    echo "  PTY 权限:       666 (所有人可读写)"
    echo ""
    echo "使用方法:"
    echo "  1. 在 LIS 模拟器串口列表中选择 $SERIAL_PORT_LIS"
    echo "  2. 运行 ./tests/test_serial_linux.sh 发送测试数据"
}

# 销毁虚拟串口
stop_vserial() {
    check_root "stop"

    if [ ! -f "$PIDFILE" ]; then
        echo -e "${YELLOW}虚拟串口未运行${NC}"
        rm -f "$SERIAL_PORT_LIS" "$SERIAL_PORT_INSTRUMENT" 2>/dev/null || true
        return 0
    fi

    local pid=$(cat "$PIDFILE")

    if kill -0 "$pid" 2>/dev/null; then
        echo "正在停止虚拟串口 (PID: $pid)..."
        kill "$pid"
        wait "$pid" 2>/dev/null || true
        echo -e "${GREEN}虚拟串口已停止${NC}"
    else
        echo -e "${YELLOW}socat 进程已不存在${NC}"
    fi

    rm -f "$PIDFILE"
    rm -f "$SERIAL_PORT_LIS" "$SERIAL_PORT_INSTRUMENT"
    echo "已清理符号链接"
}

# 查看状态
status_vserial() {
    if [ ! -f "$PIDFILE" ]; then
        echo -e "${YELLOW}虚拟串口未运行${NC}"
        if [ -L "$SERIAL_PORT_LIS" ] || [ -L "$SERIAL_PORT_INSTRUMENT" ]; then
            echo -e "${YELLOW}发现残留符号链接，建议清理:${NC}"
            echo "  sudo rm -f $SERIAL_PORT_LIS $SERIAL_PORT_INSTRUMENT"
        fi
        return 0
    fi

    local pid=$(cat "$PIDFILE")

    if kill -0 "$pid" 2>/dev/null; then
        echo -e "${GREEN}虚拟串口运行中${NC}"
        echo "  socat PID:      $pid"
        echo "  LIS 端口:       $SERIAL_PORT_LIS"
        echo "  仪器端口:       $SERIAL_PORT_INSTRUMENT"

        if [ -L "$SERIAL_PORT_LIS" ]; then
            local real_lis=$(readlink -f "$SERIAL_PORT_LIS")
            echo "  LIS 实际设备:   $real_lis"
        fi
        if [ -L "$SERIAL_PORT_INSTRUMENT" ]; then
            local real_instr=$(readlink -f "$SERIAL_PORT_INSTRUMENT")
            echo "  仪器实际设备:   $real_instr"
        fi
    else
        echo -e "${YELLOW}socat 进程已不存在，清理 PID 文件${NC}"
        rm -f "$PIDFILE"
    fi
}

# 主逻辑
case "${1:-}" in
    start)
        start_vserial
        ;;
    stop)
        stop_vserial
        ;;
    status)
        status_vserial
        ;;
    *)
        echo "用法: sudo $0 {start|stop|status}"
        echo ""
        echo "命令:"
        echo "  start   - 创建虚拟串口 ($SERIAL_PORT_LIS <-> $SERIAL_PORT_INSTRUMENT)"
        echo "  stop    - 销毁虚拟串口"
        echo "  status  - 查看虚拟串口状态"
        exit 1
        ;;
esac
