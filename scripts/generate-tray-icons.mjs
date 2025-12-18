/**
 * 生成托盘图标脚本
 * 
 * 创建四种状态的托盘图标：
 * - tray-running.png: 绿色圆形（正常运行）
 * - tray-warning.png: 黄色圆形（警告状态）
 * - tray-error.png: 红色圆形（错误状态）
 * - tray-stopped.png: 灰色圆形（停止状态）
 * 
 * 对于 macOS，还会生成模板图标以适应深色/浅色模式
 */

import sharp from 'sharp';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { mkdirSync, existsSync } from 'fs';

const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

const ICON_SIZE = 32;
const OUTPUT_DIR = join(__dirname, '..', 'src-tauri', 'icons', 'tray');

// 确保输出目录存在
if (!existsSync(OUTPUT_DIR)) {
  mkdirSync(OUTPUT_DIR, { recursive: true });
}

/**
 * 创建圆形图标的 SVG
 * @param {string} color - 填充颜色
 * @param {boolean} isTemplate - 是否为模板图标（macOS 深色/浅色模式）
 */
function createCircleSvg(color, isTemplate = false) {
  const fillColor = isTemplate ? '#000000' : color;
  const opacity = isTemplate ? '1' : '1';
  
  return `
    <svg width="${ICON_SIZE}" height="${ICON_SIZE}" viewBox="0 0 ${ICON_SIZE} ${ICON_SIZE}" xmlns="http://www.w3.org/2000/svg">
      <circle cx="${ICON_SIZE / 2}" cy="${ICON_SIZE / 2}" r="${ICON_SIZE / 2 - 2}" fill="${fillColor}" fill-opacity="${opacity}"/>
    </svg>
  `.trim();
}

/**
 * 创建带边框的圆形图标 SVG（更好的可见性）
 * @param {string} fillColor - 填充颜色
 * @param {string} strokeColor - 边框颜色
 */
function createCircleWithBorderSvg(fillColor, strokeColor) {
  return `
    <svg width="${ICON_SIZE}" height="${ICON_SIZE}" viewBox="0 0 ${ICON_SIZE} ${ICON_SIZE}" xmlns="http://www.w3.org/2000/svg">
      <circle cx="${ICON_SIZE / 2}" cy="${ICON_SIZE / 2}" r="${ICON_SIZE / 2 - 3}" fill="${fillColor}" stroke="${strokeColor}" stroke-width="2"/>
    </svg>
  `.trim();
}

/**
 * 生成图标文件
 * @param {string} name - 文件名（不含扩展名）
 * @param {string} svg - SVG 内容
 */
async function generateIcon(name, svg) {
  const outputPath = join(OUTPUT_DIR, `${name}.png`);
  
  await sharp(Buffer.from(svg))
    .resize(ICON_SIZE, ICON_SIZE)
    .png()
    .toFile(outputPath);
  
  console.log(`✓ 生成图标: ${outputPath}`);
}

/**
 * 生成 macOS 模板图标（@2x 版本）
 * @param {string} name - 文件名（不含扩展名）
 * @param {string} svg - SVG 内容
 */
async function generateTemplateIcon(name, svg) {
  // 标准尺寸
  const outputPath = join(OUTPUT_DIR, `${name}Template.png`);
  await sharp(Buffer.from(svg))
    .resize(ICON_SIZE, ICON_SIZE)
    .png()
    .toFile(outputPath);
  console.log(`✓ 生成模板图标: ${outputPath}`);
  
  // @2x 版本
  const output2xPath = join(OUTPUT_DIR, `${name}Template@2x.png`);
  await sharp(Buffer.from(svg))
    .resize(ICON_SIZE * 2, ICON_SIZE * 2)
    .png()
    .toFile(output2xPath);
  console.log(`✓ 生成模板图标 @2x: ${output2xPath}`);
}

async function main() {
  console.log('开始生成托盘图标...\n');
  
  // 定义图标颜色
  const icons = [
    { name: 'tray-running', fill: '#22c55e', stroke: '#16a34a' },   // 绿色
    { name: 'tray-warning', fill: '#eab308', stroke: '#ca8a04' },   // 黄色
    { name: 'tray-error', fill: '#ef4444', stroke: '#dc2626' },     // 红色
    { name: 'tray-stopped', fill: '#9ca3af', stroke: '#6b7280' },   // 灰色
  ];
  
  // 生成彩色图标
  for (const icon of icons) {
    const svg = createCircleWithBorderSvg(icon.fill, icon.stroke);
    await generateIcon(icon.name, svg);
  }
  
  console.log('\n生成 macOS 模板图标...\n');
  
  // 生成 macOS 模板图标（黑色，系统会自动适应深色/浅色模式）
  const templateSvg = createCircleSvg('#000000', true);
  await generateTemplateIcon('tray', templateSvg);
  
  console.log('\n✅ 所有图标生成完成！');
  console.log(`📁 输出目录: ${OUTPUT_DIR}`);
}

main().catch(console.error);
