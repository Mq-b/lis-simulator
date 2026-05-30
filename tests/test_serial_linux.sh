#!/bin/bash
# LIS 模拟器 Linux 串口测试脚本
# 通过虚拟串口向 LIS 模拟器发送 ASTM 测试数据
#
# 用法:
#   ./tests/test_serial_linux.sh [端口] [波特率]
#   ./tests/test_serial_linux.sh                    # 默认 /dev/ttyV1 9600
#   ./tests/test_serial_linux.sh /dev/ttyV1 115200  # 自定义

set -e

# 默认配置 - 使用 /dev 目录，LIS 模拟器可以识别
DEFAULT_PORT="/dev/ttyV1"
DEFAULT_BAUD=9600

# 显示帮助
show_help() {
    echo "LIS 模拟器 Linux 串口测试脚本"
    echo ""
    echo "用法:"
    echo "  $0 [端口] [波特率]"
    echo "  $0                           # 默认 /dev/ttyV1 9600"
    echo "  $0 /dev/ttyV1 115200         # 自定义"
    echo ""
    echo "参数:"
    echo "  端口    串口设备路径 (默认: /dev/ttyV1)"
    echo "  波特率  通信波特率 (默认: 9600)"
    echo ""
    echo "示例:"
    echo "  # 使用虚拟串口 (需要 root 权限创建)"
    echo "  sudo ./tests/setup_vserial.sh start"
    echo "  $0"
    echo ""
    echo "  # 使用物理串口"
    echo "  $0 /dev/ttyUSB0 9600"
    exit 0
}

# 处理 --help 参数
if [ "${1:-}" = "--help" ] || [ "${1:-}" = "-h" ]; then
    show_help
fi

# 参数
PORT="${1:-$DEFAULT_PORT}"
BAUD="${2:-$DEFAULT_BAUD}"

# 脚本目录
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# 检查依赖
check_dependencies() {
    local missing=0

    # 检查 python3
    if ! command -v python3 &> /dev/null; then
        echo -e "${RED}错误: python3 未安装${NC}"
        missing=1
    fi

    # 检查 pyserial
    if ! python3 -c "import serial" 2>/dev/null; then
        echo -e "${RED}错误: pyserial 未安装${NC}"
        echo "请运行: pip3 install pyserial"
        missing=1
    fi

    # 检查仪器模拟器脚本
    if [ ! -f "$SCRIPT_DIR/instrument_simulator.py" ]; then
        echo -e "${RED}错误: instrument_simulator.py 不存在${NC}"
        missing=1
    fi

    # 检查协议配置
    if [ ! -f "$SCRIPT_DIR/protocol_config.py" ]; then
        echo -e "${RED}错误: protocol_config.py 不存在${NC}"
        missing=1
    fi

    if [ $missing -eq 1 ]; then
        exit 1
    fi
}

# 检查虚拟串口
check_vserial() {
    if [ ! -e "$PORT" ]; then
        echo -e "${RED}错误: 串口 $PORT 不存在${NC}"
        echo ""
        echo "请先创建虚拟串口 (需要 root 权限):"
        echo "  sudo ./tests/setup_vserial.sh start"
        echo ""
        echo "或者指定其他端口:"
        echo "  $0 /dev/ttyUSB0 9600"
        exit 1
    fi
}

# 运行仪器模拟器
run_simulator() {
    echo -e "${CYAN}========================================${NC}"
    echo -e "${CYAN}  LIS 模拟器 - 串口测试${NC}"
    echo -e "${CYAN}========================================${NC}"
    echo ""
    echo "配置:"
    echo "  串口: $PORT"
    echo "  波特率: $BAUD"
    echo ""

    # 检查是否是虚拟串口
    if [[ "$PORT" == /dev/ttyV* ]]; then
        echo -e "${YELLOW}提示: 请确保 LIS 模拟器已连接到对应的虚拟串口${NC}"
        if [ "$PORT" = "/dev/ttyV1" ]; then
            echo "      LIS 模拟器应选择 /dev/ttyV0"
        elif [ "$PORT" = "/dev/ttyV0" ]; then
            echo "      LIS 模拟器应选择 /dev/ttyV1"
        fi
        echo ""
    fi

    echo "正在发送 ASTM 测试数据..."
    echo ""

    # 运行仪器模拟器
    cd "$SCRIPT_DIR"
    python3 instrument_simulator.py --port "$PORT" --baud "$BAUD" --mode single

    echo ""
    echo -e "${GREEN}测试完成${NC}"
}

# 主流程
main() {
    # 检查依赖
    check_dependencies

    # 检查串口
    check_vserial

    # 运行模拟器
    run_simulator
}

main
