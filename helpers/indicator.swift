// j-indicator — 屏幕点击光圈指示器
// 用法: j-indicator <x> <y> [label]
// 在指定坐标显示红色光圈动画，可选文字标注

import Cocoa

// ── 参数解析 ──────────────────────────────────────────────
let args = CommandLine.arguments
guard args.count >= 3,
      let x = Double(args[1]),
      let y = Double(args[2]) else {
    fputs("用法: j-indicator <x> <y> [label]\n", stderr)
    exit(1)
}
let label: String? = args.count >= 4 ? args[3] : nil

// ── 常量 ─────────────────────────────────────────────────
let circleSize: CGFloat  = 48
let lineWidth: CGFloat   = 3
let showDuration: Double  = 0.8
let fadeDuration: Double  = 0.35

// ── Application 初始化 ───────────────────────────────────
let app = NSApplication.shared
app.setActivationPolicy(.accessory)   // 不在 Dock 中显示

// ── 光圈 View ────────────────────────────────────────────
class IndicatorView: NSView {
    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        guard let ctx = NSGraphicsContext.current?.cgContext else { return }

        // 红色光圈
        let inset = lineWidth / 2
        let circleRect = NSRect(
            x: inset, y: bounds.height - circleSize - inset,
            width: circleSize - lineWidth, height: circleSize - lineWidth
        )
        ctx.setStrokeColor(NSColor.systemRed.cgColor)
        ctx.setLineWidth(lineWidth)
        ctx.strokeEllipse(in: circleRect)

        // 中心点
        let dotSize: CGFloat = 5
        let cx = circleRect.midX - dotSize / 2
        let cy = circleRect.midY - dotSize / 2
        ctx.setFillColor(NSColor.systemRed.cgColor)
        ctx.fillEllipse(in: NSRect(x: cx, y: cy, width: dotSize, height: dotSize))
    }
}

// ── 标注 View ────────────────────────────────────────────
class LabelView: NSView {
    let text: String
    init(text: String, frame: NSRect) {
        self.text = text
        super.init(frame: frame)
    }
    required init?(coder: NSCoder) { fatalError() }

    override func draw(_ dirtyRect: NSRect) {
        super.draw(dirtyRect)
        let paragraphStyle = NSMutableParagraphStyle()
        paragraphStyle.alignment = .center
        let attrs: [NSAttributedString.Key: Any] = [
            .font: NSFont.systemFont(ofSize: 11, weight: .semibold),
            .foregroundColor: NSColor.black,
            .paragraphStyle: paragraphStyle
        ]
        // 白色圆角背景
        let bgPath = NSBezierPath(roundedRect: bounds.insetBy(dx: 0.5, dy: 0.5),
                                   xRadius: 4, yRadius: 4)
        NSColor.white.withAlphaComponent(0.92).setFill()
        bgPath.fill()
        NSColor.black.withAlphaComponent(0.15).setStroke()
        bgPath.stroke()

        let textRect = bounds.insetBy(dx: 4, dy: 2)
        (text as NSString).draw(in: textRect, withAttributes: attrs)
    }
}

// ── 计算屏幕坐标（CoreGraphics → AppKit 翻转 Y）────────
let screenHeight = NSScreen.main?.frame.height ?? 900
let appKitY = screenHeight - CGFloat(y)

// ── 光圈窗口 ─────────────────────────────────────────────
let circleOrigin = NSPoint(
    x: CGFloat(x) - circleSize / 2,
    y: appKitY - circleSize / 2
)
let circleWindow = NSWindow(
    contentRect: NSRect(origin: circleOrigin,
                        size: NSSize(width: circleSize, height: circleSize)),
    styleMask: .borderless,
    backing: .buffered,
    defer: false
)
circleWindow.isOpaque = false
circleWindow.backgroundColor = .clear
circleWindow.level = .screenSaver
circleWindow.ignoresMouseEvents = true
circleWindow.hasShadow = false
circleWindow.contentView = IndicatorView(
    frame: NSRect(origin: .zero, size: NSSize(width: circleSize, height: circleSize))
)

circleWindow.orderFrontRegardless()
circleWindow.alphaValue = 1.0

// ── 标注窗口（可选）──────────────────────────────────────
var labelWindow: NSWindow? = nil
if let labelText = label {
    let attrs: [NSAttributedString.Key: Any] = [
        .font: NSFont.systemFont(ofSize: 11, weight: .semibold)
    ]
    let textSize = (labelText as NSString).size(withAttributes: attrs)
    let padding: CGFloat = 10
    let labelWidth = textSize.width + padding * 2
    let labelHeight: CGFloat = 20

    let labelOrigin = NSPoint(
        x: CGFloat(x) + circleSize / 2 + 4,
        y: appKitY + circleSize / 2 - labelHeight + 4
    )
    let win = NSWindow(
        contentRect: NSRect(origin: labelOrigin,
                            size: NSSize(width: labelWidth, height: labelHeight)),
        styleMask: .borderless,
        backing: .buffered,
        defer: false
    )
    win.isOpaque = false
    win.backgroundColor = .clear
    win.level = .screenSaver
    win.ignoresMouseEvents = true
    win.hasShadow = false
    win.contentView = LabelView(
        text: labelText,
        frame: NSRect(origin: .zero, size: NSSize(width: labelWidth, height: labelHeight))
    )
    win.orderFrontRegardless()
    win.alphaValue = 1.0
    labelWindow = win
}

// ── 动画：停留 → 淡出 → 退出 ────────────────────────────
DispatchQueue.main.asyncAfter(deadline: .now() + showDuration) {
    NSAnimationContext.runAnimationGroup({ ctx in
        ctx.duration = fadeDuration
        circleWindow.animator().alphaValue = 0
        labelWindow?.animator().alphaValue = 0
    }, completionHandler: {
        NSApp.terminate(nil)
    })
}

app.run()
