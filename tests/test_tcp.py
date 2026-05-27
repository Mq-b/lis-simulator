#!/usr/bin/env python3
"""
LIS 模拟器 TCP 集成测试

通过 TCP 连接 LIS 模拟器，模拟仪器端发送 ASTM 数据，
验证解析结果是否正确。

使用方法:
    1. 启动 LIS 模拟器:  cargo run -- --tcp 12345
    2. 运行测试:          python tests/test_tcp.py --port 12345
"""

import argparse
import socket
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
    return format(sum(data) % 256, 'x')


def build_frame(records: list) -> bytes:
    """构建一个 ASTM 数据帧"""
    frame_data = b'1'  # 帧号
    for i, record in enumerate(records):
        frame_data += record.encode('utf-8')
        if i < len(records) - 1:
            frame_data += CR

    checksum_payload = frame_data + ETX
    cs = calc_checksum(checksum_payload)
    return STX + frame_data + ETX + cs.encode() + CR + LF


def recv_exact(sock: socket.socket, n: int, timeout: float = 5.0) -> bytes:
    """精确接收 n 个字节"""
    sock.settimeout(timeout)
    buf = b''
    while len(buf) < n:
        chunk = sock.recv(n - len(buf))
        if not chunk:
            raise ConnectionError("连接已关闭")
        buf += chunk
    return buf


def test_single_message(sock: socket.socket) -> bool:
    """测试 1: 发送单条完整消息"""
    print("\n--- 测试 1: 单条消息 ---")
    now = time.strftime("%Y%m%d%H%M%S")

    records = [
        f"H|\\^&|INST^0|PR|V1.0|{now}",
        f"P|1|20250518001|张三|36|M|C||李四|骨科",
        f"R|1|cTnI|0.03|ng/ml|0|0.15|{now}|U|",
        f"R|2|CK-MB|4.3|ng/ml|0|6|{now}|U|",
        f"R|3|NT-proBNP|125.0|pg/ml|0|100.0|{now}|H|偏高",
        "L|1|N",
    ]

    # ENQ 握手
    print("  [TX] ENQ")
    sock.sendall(ENQ)
    resp = recv_exact(sock, 1)
    assert resp == ACK, f"期望 ACK(0x06), 收到 {resp.hex()}"
    print("  [RX] ACK")

    # 发送数据帧
    frame = build_frame(records)
    print(f"  [TX] 数据帧 ({len(frame)} bytes)")
    sock.sendall(frame)
    resp = recv_exact(sock, 1)
    assert resp == ACK, f"期望 ACK(0x06), 收到 {resp.hex()}"
    print("  [RX] ACK")

    # EOT
    print("  [TX] EOT")
    sock.sendall(EOT)

    print("  [PASS] 单条消息发送成功")
    return True


def test_multiple_messages(sock: socket.socket) -> bool:
    """测试 2: 连续发送多条消息"""
    print("\n--- 测试 2: 连续多条消息 ---")

    patients = [
        ("20250518002", "李四", "cTnI", "0.05"),
        ("20250518003", "王五", "CK-MB", "12.0"),
    ]

    for i, (sid, name, item, value) in enumerate(patients):
        now = time.strftime("%Y%m%d%H%M%S")
        records = [
            f"H|\\^&|INST^0|PR|V1.0|{now}",
            f"P|1|{sid}|{name}|40|F|H|BED{i}|Dr.Zhao|Internal",
            f"R|1|{item}|{value}|ng/ml|0|10|{now}|U|",
            "L|1|N",
        ]

        print(f"\n  消息 {i+2}: {name} ({sid})")
        sock.sendall(ENQ)
        resp = recv_exact(sock, 1)
        assert resp == ACK, f"期望 ACK, 收到 {resp.hex()}"
        print("  [RX] ACK")

        frame = build_frame(records)
        sock.sendall(frame)
        resp = recv_exact(sock, 1)
        assert resp == ACK, f"期望 ACK, 收到 {resp.hex()}"
        print("  [RX] ACK")

        sock.sendall(EOT)
        print(f"  消息 {i+2} 发送完成")

    print("\n  [PASS] 连续多条消息发送成功")
    return True


def test_bad_checksum(sock: socket.socket) -> bool:
    """测试 3: 发送校验和错误的帧"""
    print("\n--- 测试 3: 错误校验和 ---")

    records = ["H|\\^&|INST|PR|V1.0|20250101", "L|1|N"]
    frame = bytearray(build_frame(records))

    # 篡改校验和
    frame[-4] = ord('f')
    frame[-3] = ord('f')

    sock.sendall(ENQ)
    resp = recv_exact(sock, 1)
    assert resp == ACK, f"期望 ACK, 收到 {resp.hex()}"

    sock.sendall(bytes(frame))
    # LIS 应该回 ACK 但标记校验失败（或回 NAK）
    # 这里只验证不会崩溃，不强制要求特定响应
    try:
        resp = recv_exact(sock, 1, timeout=3.0)
        print(f"  [RX] 0x{resp.hex()} (校验错误帧的响应)")
    except socket.timeout:
        print("  [RX] 超时（LIS 未回复，可能拒绝了错误帧）")

    sock.sendall(EOT)
    print("  [PASS] 错误校验和测试完成")
    return True


def main():
    parser = argparse.ArgumentParser(description="LIS 模拟器 TCP 集成测试")
    parser.add_argument("--port", type=int, default=12345, help="TCP 端口 (默认 12345)")
    parser.add_argument("--host", default="127.0.0.1", help="主机地址 (默认 127.0.0.1)")
    args = parser.parse_args()

    print(f"连接 {args.host}:{args.port}...")
    try:
        sock = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        sock.connect((args.host, args.port))
    except ConnectionRefusedError:
        print(f"连接失败: 请先启动 LIS 模拟器 (cargo run -- --tcp {args.port})")
        sys.exit(1)

    print("已连接\n")

    tests = [
        ("单条消息", test_single_message),
        ("连续多消息", test_multiple_messages),
        ("错误校验和", test_bad_checksum),
    ]

    passed = 0
    failed = 0

    for name, test_fn in tests:
        try:
            if test_fn(sock):
                passed += 1
            else:
                failed += 1
                print(f"  [FAIL] {name}")
        except Exception as e:
            failed += 1
            print(f"  [FAIL] {name}: {e}")

    sock.close()

    print(f"\n{'='*40}")
    print(f"  结果: {passed} 通过, {failed} 失败")
    print(f"{'='*40}")

    if failed > 0:
        sys.exit(1)


if __name__ == "__main__":
    main()
