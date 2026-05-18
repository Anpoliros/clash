"""
我们来写一个python脚本，将html转成markdown，使用markdownify。
用法
python remark.py -i 1.html -o 1.md
"""

import argparse
import sys
import os

try:
    from markdownify import markdownify as md
except ImportError:
    print("错误: 缺少 markdownify 库。请先运行: pip install markdownify")
    sys.exit(1)

#----主函数----
def main():
    # 设置命令行参数解析
    parser = argparse.ArgumentParser(description="将 HTML 转换为 Markdown")
    parser.add_argument("-i", "--input", required=True, help="输入的 HTML 文件路径")
    parser.add_argument("-o", "--output", required=True, help="输出的 Markdown 文件路径")
    
    args = parser.parse_args()
    
    # 检查输入文件是否存在
    if not os.path.exists(args.input):
        print(f"错误: 输入文件 '{args.input}' 不存在。")
        sys.exit(1)
        
    try:
        # 读取 HTML 文件
        with open(args.input, 'r', encoding='utf-8') as f:
            html_content = f.read()
            
        # 转换为 Markdown (使用 ATX 风格的标题，即 # 标题)
        markdown_content = md(html_content, heading_style="ATX")
        
        # 写入 Markdown 文件
        with open(args.output, 'w', encoding='utf-8') as f:
            f.write(markdown_content)
            
        print(f"成功: '{args.input}' 已转换并保存至 '{args.output}'")
        
    except Exception as e:
        print(f"转换过程中发生错误: {e}")
        sys.exit(1)

if __name__ == "__main__":
    main()