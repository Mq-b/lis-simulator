#!/usr/bin/env python3
"""
ASTM 协议配置加载

从 settings/protocol.json 读取协议参数，供测试脚本使用。
"""

import json
from pathlib import Path


def load_config() -> dict:
    """加载协议配置"""
    # 尝试多个路径
    search_paths = [
        Path(__file__).parent.parent / "settings" / "protocol.json",
        Path("settings") / "protocol.json",
        Path("protocol.json"),
    ]

    for path in search_paths:
        if path.exists():
            with open(path, "r", encoding="utf-8") as f:
                return json.load(f)

    # 默认配置（标准 ASTM）
    return {
        "astm": {
            "has_frame_number": True,
            "checksum_includes_stx": False,
            "checksum_zero_padded": True,
        }
    }


# 全局配置实例
CONFIG = load_config()
ASTM_CONFIG = CONFIG.get("astm", {})

# 导出配置项
HAS_FRAME_NUMBER = ASTM_CONFIG.get("has_frame_number", True)
CHECKSUM_INCLUDES_STX = ASTM_CONFIG.get("checksum_includes_stx", False)
CHECKSUM_ZERO_PADDED = ASTM_CONFIG.get("checksum_zero_padded", True)


def calc_checksum(data: bytes) -> str:
    """计算校验和"""
    cs = sum(data) % 256
    if CHECKSUM_ZERO_PADDED:
        return format(cs, '02x')
    return format(cs, 'x')


def build_frame(records: list[str]) -> bytes:
    """构建 ASTM 数据帧"""
    STX = b'\x02'
    ETX = b'\x03'
    CR = b'\x0D'
    LF = b'\x0A'

    frame_data = b''

    # 添加帧号（如果配置要求）
    if HAS_FRAME_NUMBER:
        frame_data += b'1'

    # 添加记录
    for i, record in enumerate(records):
        frame_data += record.encode('utf-8')
        if i < len(records) - 1:
            frame_data += CR

    # 计算校验和
    if CHECKSUM_INCLUDES_STX:
        checksum_payload = STX + frame_data + ETX
    else:
        checksum_payload = frame_data + ETX

    cs = calc_checksum(checksum_payload)

    return STX + frame_data + ETX + cs.encode() + CR + LF


if __name__ == "__main__":
    print(f"HAS_FRAME_NUMBER: {HAS_FRAME_NUMBER}")
    print(f"CHECKSUM_INCLUDES_STX: {CHECKSUM_INCLUDES_STX}")
    print(f"CHECKSUM_ZERO_PADDED: {CHECKSUM_ZERO_PADDED}")

    # 测试构建帧
    frame = build_frame(["H|\\^&|TEST|PR|V1.0", "L|1|N"])
    print(f"\n测试帧 ({len(frame)} bytes):")
    print(frame.hex())
