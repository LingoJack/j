// aic-ax — Accessibility API helper for aic
// Usage:
//   aic-ax tree [--app <name>] [--depth <n>] [--clickable]
//   aic-ax find <text> [--app <name>] [--role <role>]

import Cocoa
import ApplicationServices

// MARK: - Data structures

struct Frame: Codable {
    let x: Double
    let y: Double
    let w: Double
    let h: Double
}

struct AxNode: Codable {
    let role: String
    let title: String?
    let description: String?
    let value: String?
    let frame: Frame?
    let enabled: Bool?
    var children: [AxNode]?
}

struct FindResult: Codable {
    let role: String
    let title: String?
    let description: String?
    let value: String?
    let frame: Frame?
    let center_x: Double
    let center_y: Double
}

// MARK: - Accessibility helpers

let interactiveRoles: Set<String> = [
    "AXButton", "AXCheckBox", "AXRadioButton", "AXPopUpButton",
    "AXMenuButton", "AXTextField", "AXTextArea", "AXSecureTextField",
    "AXSlider", "AXIncrementor", "AXComboBox", "AXColorWell",
    "AXLink", "AXDisclosureTriangle", "AXTab", "AXMenuItem",
    "AXToolbarButton", "AXSegmentedControl",
]

func getStringAttr(_ element: AXUIElement, _ attr: String) -> String? {
    var value: AnyObject?
    let result = AXUIElementCopyAttributeValue(element, attr as CFString, &value)
    guard result == .success, let str = value as? String else { return nil }
    return str
}

func getBoolAttr(_ element: AXUIElement, _ attr: String) -> Bool? {
    var value: AnyObject?
    let result = AXUIElementCopyAttributeValue(element, attr as CFString, &value)
    guard result == .success else { return nil }
    if let num = value as? NSNumber {
        return num.boolValue
    }
    return nil
}

func getFrame(_ element: AXUIElement) -> Frame? {
    var posValue: AnyObject?
    var sizeValue: AnyObject?

    guard AXUIElementCopyAttributeValue(element, kAXPositionAttribute as CFString, &posValue) == .success,
          AXUIElementCopyAttributeValue(element, kAXSizeAttribute as CFString, &sizeValue) == .success
    else { return nil }

    var point = CGPoint.zero
    var size = CGSize.zero

    guard AXValueGetValue(posValue as! AXValue, .cgPoint, &point),
          AXValueGetValue(sizeValue as! AXValue, .cgSize, &size)
    else { return nil }

    return Frame(x: Double(point.x), y: Double(point.y), w: Double(size.width), h: Double(size.height))
}

func getChildren(_ element: AXUIElement) -> [AXUIElement] {
    var value: AnyObject?
    let result = AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &value)
    guard result == .success, let children = value as? [AXUIElement] else { return [] }
    return children
}

// MARK: - Tree building

func buildTree(_ element: AXUIElement, depth: Int, maxDepth: Int, clickableOnly: Bool) -> AxNode? {
    let role = getStringAttr(element, kAXRoleAttribute) ?? "AXUnknown"
    let title = getStringAttr(element, kAXTitleAttribute)
    let desc = getStringAttr(element, kAXDescriptionAttribute)
    let value = getStringAttr(element, kAXValueAttribute)
    let frame = getFrame(element)
    let enabled = getBoolAttr(element, kAXEnabledAttribute)

    var childNodes: [AxNode]? = nil

    if depth < maxDepth {
        let children = getChildren(element)
        if !children.isEmpty {
            var built: [AxNode] = []
            for child in children {
                if let node = buildTree(child, depth: depth + 1, maxDepth: maxDepth, clickableOnly: clickableOnly) {
                    built.append(node)
                }
            }
            if !built.isEmpty {
                childNodes = built
            }
        }
    }

    // In clickable-only mode, prune non-interactive leaves
    if clickableOnly {
        let isInteractive = interactiveRoles.contains(role)
        let hasInteractiveChildren = childNodes != nil && !childNodes!.isEmpty
        if !isInteractive && !hasInteractiveChildren {
            return nil
        }
    }

    return AxNode(role: role, title: title, description: desc, value: value, frame: frame, enabled: enabled, children: childNodes)
}

// MARK: - Find elements

func findElements(_ element: AXUIElement, query: String, roleFilter: String?, results: inout [FindResult]) {
    let role = getStringAttr(element, kAXRoleAttribute) ?? "AXUnknown"
    let title = getStringAttr(element, kAXTitleAttribute)
    let desc = getStringAttr(element, kAXDescriptionAttribute)
    let value = getStringAttr(element, kAXValueAttribute)

    // Check role filter
    if let filter = roleFilter, role != filter {
        // Still search children
    } else {
        let queryLower = query.lowercased()
        let matches = [title, desc, value].compactMap { $0 }.contains { $0.lowercased().contains(queryLower) }

        if matches {
            let frame = getFrame(element)
            let cx: Double
            let cy: Double
            if let f = frame {
                cx = f.x + f.w / 2.0
                cy = f.y + f.h / 2.0
            } else {
                cx = 0
                cy = 0
            }
            results.append(FindResult(role: role, title: title, description: desc, value: value, frame: frame, center_x: cx, center_y: cy))
        }
    }

    for child in getChildren(element) {
        findElements(child, query: query, roleFilter: roleFilter, results: &results)
    }
}

// MARK: - Collect interactive elements (for SoM)

func collectInteractive(_ element: AXUIElement, results: inout [FindResult]) {
    let role = getStringAttr(element, kAXRoleAttribute) ?? "AXUnknown"

    if interactiveRoles.contains(role) {
        let title = getStringAttr(element, kAXTitleAttribute)
        let desc = getStringAttr(element, kAXDescriptionAttribute)
        let value = getStringAttr(element, kAXValueAttribute)
        let frame = getFrame(element)

        // Only include elements with a valid frame and non-zero size
        if let f = frame, f.w > 0 && f.h > 0 {
            let cx = f.x + f.w / 2.0
            let cy = f.y + f.h / 2.0
            results.append(FindResult(role: role, title: title, description: desc, value: value, frame: frame, center_x: cx, center_y: cy))
        }
    }

    for child in getChildren(element) {
        collectInteractive(child, results: &results)
    }
}

// MARK: - Target app resolution

func getFrontmostApp() -> NSRunningApplication? {
    return NSWorkspace.shared.frontmostApplication
}

func findAppByName(_ name: String) -> NSRunningApplication? {
    let apps = NSWorkspace.shared.runningApplications
    let nameLower = name.lowercased()
    return apps.first { app in
        if let n = app.localizedName, n.lowercased() == nameLower {
            return true
        }
        if let n = app.bundleIdentifier, n.lowercased().contains(nameLower) {
            return true
        }
        return false
    }
}

func getAppElement(_ appName: String?) -> AXUIElement? {
    let app: NSRunningApplication?
    if let name = appName {
        app = findAppByName(name)
        if app == nil {
            fputs("Error: application '\(name)' not found\n", stderr)
            return nil
        }
    } else {
        app = getFrontmostApp()
        if app == nil {
            fputs("Error: no frontmost application\n", stderr)
            return nil
        }
    }
    return AXUIElementCreateApplication(app!.processIdentifier)
}

// MARK: - Main

guard AXIsProcessTrusted() else {
    fputs("Error: Accessibility permission required. Grant access in System Settings > Privacy & Security > Accessibility.\n", stderr)
    exit(2)
}

let args = Array(CommandLine.arguments.dropFirst())

guard !args.isEmpty else {
    fputs("Usage: aic-ax tree [--app <name>] [--depth <n>] [--clickable]\n       aic-ax find <text> [--app <name>] [--role <role>]\n       aic-ax interactive [--app <name>]\n", stderr)
    exit(1)
}

let subcommand = args[0]

func parseArg(_ flag: String) -> String? {
    if let idx = args.firstIndex(of: flag), idx + 1 < args.count {
        return args[idx + 1]
    }
    return nil
}

func hasFlag(_ flag: String) -> Bool {
    return args.contains(flag)
}

let encoder = JSONEncoder()
encoder.outputFormatting = [.sortedKeys]

switch subcommand {
case "tree":
    let appName = parseArg("--app")
    let maxDepth = Int(parseArg("--depth") ?? "10") ?? 10
    let clickableOnly = hasFlag("--clickable")

    guard let appElement = getAppElement(appName) else { exit(1) }

    if let tree = buildTree(appElement, depth: 0, maxDepth: maxDepth, clickableOnly: clickableOnly) {
        if let data = try? encoder.encode(tree) {
            print(String(data: data, encoding: .utf8)!)
        }
    } else {
        print("{}")
    }

case "find":
    guard args.count >= 2 else {
        fputs("Error: 'find' requires a search text argument\n", stderr)
        exit(1)
    }
    let query = args[1]
    let appName = parseArg("--app")
    let roleFilter = parseArg("--role")

    guard let appElement = getAppElement(appName) else { exit(1) }

    var results: [FindResult] = []
    findElements(appElement, query: query, roleFilter: roleFilter, results: &results)

    if let data = try? encoder.encode(results) {
        print(String(data: data, encoding: .utf8)!)
    }

case "interactive":
    let appName = parseArg("--app")

    guard let appElement = getAppElement(appName) else { exit(1) }

    var results: [FindResult] = []
    collectInteractive(appElement, results: &results)

    if let data = try? encoder.encode(results) {
        print(String(data: data, encoding: .utf8)!)
    }

default:
    fputs("Error: unknown subcommand '\(subcommand)'. Use 'tree', 'find', or 'interactive'.\n", stderr)
    exit(1)
}
