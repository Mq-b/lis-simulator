#!/usr/bin/env python3
"""
ASTM 仪器模拟器
模拟医疗检验仪器向 LIS 发送 ASTM E1381/E1394 协议数据。
用于测试 LIS 模拟器的接收和解析功能。

使用方法:
    python instrument_simulator.py --port COM11 --baud 9600
    python instrument_simulator.py --port COM11 --mode bidirectional
"""

import argparse
import serial
import time
import sys

# ASTM 控制字符
ENQ = b'\x05'
ACK = b'\x06'
NAK = b'\x15'
EOT = b'\x04'
STX = b'\x02'
ETX = b'\x03'
CR = b'\x0D'
LF = b'\x0A'


def calc_checksum(data: bytes) -> str:
    """计算校验和 (模256，hex不补零)"""
    cs = sum(data) % 256
    return format(cs, 'x')


def build_frame(records: list[str]) -> bytes:
    """构建一个 ASTM 数据帧"""
    frame_data = b'1'  # 帧号
    for i, record in enumerate(records):
        frame_data += record.encode('utf-8')
        if i < len(records) - 1:
            frame_data += CR

    checksum_payload = frame_data + ETX
    cs = calc_checksum(checksum_payload)

    return STX + frame_data + ETX + cs.encode() + CR + LF


def send_and_wait_ack(ser: serial.Serial, data: bytes, timeout: float = 5.0) -> bool:
    """发送数据并等待 ACK"""
    ser.write(data)
    ser.flush()
    response = ser.read(1)
    return response == ACK


def send_result_message(ser: serial.Serial, sample_id: str = "20250518001"):
    """发送一条完整的检验结果消息 (单向模式)"""
    now = time.strftime("%Y%m%d%H%M%S")

    records = [
        f"H|\\^&|INST^0|PR|V1.0|{now}",
        f"P|1|{sample_id}|张三|36|M|C||李四|骨科",
        f"R|1|cTnI|0.03|ng/ml|0|0.15|{now}|U|",
        f"R|2|CK-MB|4.3|ng/ml|0|6|{now}|U|",
        f"R|3|NT-proBNP|125.0|pg/ml|0|100.0|{now}|H|偏高",
        "L|1|N",
    ]

    frame = build_frame(records)
    print(f"[TX] ENQ")
    if not send_and_wait_ack(ser, ENQ):
        print("[RX] 未收到 ACK，中止")
        return False

    print("[RX] ACK")
    print(f"[TX] 数据帧 ({len(frame)} bytes)")
    if not send_and_wait_ack(ser, frame):
        print("[RX] 未收到 ACK，中止")
        return False

    print("[RX] ACK")
    print("[TX] EOT")
    ser.write(EOT)
    ser.flush()
    print("[完成] 消息发送成功")
    return True


def send_query_response(ser: serial.Serial):
    """发送查询应答 (双向模式下 LIS 端的应答)"""
    now = time.strftime("%Y%m%d%H%M%S")

    records = [
        f"H|\\^&|LIS_SIM|QA|V1.0|{now}",
        "L|1|N",
    ]

    frame = build_frame(records)
    print(f"[TX] 应答帧")
    ser.write(frame)
    ser.flush()


def wait_for_enq(ser: serial.Serial, timeout: float = 30.0) -> bool:
    """等待仪器发送 ENQ"""
    print(f"[等待] 等待 ENQ... (超时 {timeout}s)")
    start = time.time()
    while time.time() - start < timeout:
        data = ser.read(1)
        if data == ENQ:
            print("[RX] ENQ")
            print("[TX] ACK")
            ser.write(ACK)
            ser.flush()
            return True
    print("[超时] 未收到 ENQ")
    return False


def receive_data(ser: serial.Serial, timeout: float = 10.0) -> list[bytes]:
    """接收完整的 ASTM 数据帧"""
    frames = []
    start = time.time()

    while time.time() - start < timeout:
        data = ser.read(1)
        if not data:
            continue

        if data == EOT:
            print("[RX] EOT - 传输结束")
            break
        elif data == STX:
            # 读取帧内容直到 LF
            buf = data
            while True:
                byte = ser.read(1)
                if byte:
                    buf += byte
                    if byte == LF:
                        break
            frames.append(buf)
            print(f"[RX] 数据帧 ({len(buf)} bytes)")
            # 回 ACK
            print("[TX] ACK")
            ser.write(ACK)
            ser.flush()

    return frames


def run_single_direction(ser: serial.Port):
    """单向模式测试：发送多条结果"""
    print("\n=== 单向模式测试 ===\n")

    samples = [
        ("20250518001", "张三"),
        ("20250518002", "李四"),
        ("20250518003", "王五"),
    ]

    for i, (sid, name) in enumerate(samples):
        print(f"\n--- 消息 {i+1}: {name} ({sid}) ---")
        if not send_result_message(ser, sid):
            print("发送失败")
            break
        time.sleep(1)

    print("\n=== 单向模式测试完成 ===\n")


def run_bidirectional(ser: serial.Serial):
    """双向模式测试：等待查询并应答"""
    print("\n=== 双向模式测试 ===\n")
    print("等待仪器发送查询...")
    print("(请在 LIS 模拟器中输入样本ID并点击'发送查询')")

    frames = receive_data(ser, timeout=60.0)
    if frames:
        print(f"\n收到 {len(frames)} 个帧:")
        for f in frames:
            print(f"  {f}")

    print("\n=== 双向模式测试完成 ===\n")


def main():
    parser = argparse.ArgumentParser(description="ASTM 仪器模拟器")
    parser.add_argument("--port", required=True, help="串口号 (如 COM11)")
    parser.add_argument("--baud", type=int, default=9600, help="波特率 (默认 9600)")
    parser.add_argument("--mode", choices=["single", "bidirectional"], default="single",
                        help="测试模式: single=单向发送, bidirectional=双向查询")
    args = parser.parse_args()

    print(f"连接串口: {args.port} @ {args.baud}bps")
    try:
        ser = serial.Serial(port=args.port, baudrate=args.baud, timeout=1)
    except serial.SerialException as e:
        print(f"无法打开串口: {e}")
        sys.exit(1)

    print("串口已打开\n")

    try:
        if args.mode == "single":
            run_single_direction(ser)
        else:
            run_bidirectional(ser)
    except KeyboardInterrupt:
        print("\n用户中断")
    finally:
        ser.close()
        print("串口已关闭")


if __name__ == "__main__":
    main()
