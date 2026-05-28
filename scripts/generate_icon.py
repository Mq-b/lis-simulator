#!/usr/bin/env python3
"""生成 LIS 模拟器的 ICO 图标文件"""

from PIL import Image, ImageDraw, ImageFont
import os


def create_icon(output_path: str):
    """创建包含 LIS 文字的图标（高质量版本）"""
    # 使用大尺寸渲染后缩小，获得更好的抗锯齿效果
    render_size = 512
    sizes = [16, 32, 48, 64, 128, 256]

    # 在大画布上渲染
    img = Image.new('RGBA', (render_size, render_size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)

    # 绘制蓝色圆角矩形背景
    margin = render_size // 16
    radius = render_size // 5
    draw.rounded_rectangle(
        [margin, margin, render_size - margin, render_size - margin],
        radius=radius,
        fill=(37, 99, 235)  # #2563eb
    )

    # 绘制 LIS 文字（使用更大的字体）
    font_size = render_size // 2
    try:
        # 尝试使用粗体字体
        font = ImageFont.truetype("arialbd.ttf", font_size)
    except (OSError, IOError):
        try:
            font = ImageFont.truetype("Arial Bold.ttf", font_size)
        except (OSError, IOError):
            try:
                font = ImageFont.truetype("arial.ttf", font_size)
            except (OSError, IOError):
                font = ImageFont.load_default()

    text = "LIS"
    bbox = draw.textbbox((0, 0), text, font=font)
    text_width = bbox[2] - bbox[0]
    text_height = bbox[3] - bbox[1]
    x = (render_size - text_width) // 2
    y = (render_size - text_height) // 2 - bbox[1] - 10  # 稍微上移一点
    draw.text((x, y), text, fill='white', font=font)

    # 生成各种尺寸的图标
    images = []
    for size in sizes:
        resized = img.resize((size, size), Image.Resampling.LANCZOS)
        images.append(resized)

    # 保存为 ICO 文件
    # 使用 Pillow 的标准方式保存多尺寸 ICO
    images[0].save(
        output_path,
        format='ICO',
        append_images=images[1:],
        sizes=[(s, s) for s in sizes]
    )
    print(f"图标已保存到: {output_path}")
    print(f"文件大小: {os.path.getsize(output_path)} 字节")


if __name__ == '__main__':
    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_dir = os.path.dirname(script_dir)
    output = os.path.join(project_dir, 'assets', 'icon.ico')
    os.makedirs(os.path.dirname(output), exist_ok=True)
    create_icon(output)