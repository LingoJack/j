---
name: swift-ios-app-gen
description: ios 应用 swift 原生开发技能包；当用户描述一个开发 ios 原生应用的需求时，加载此技能
---

iOS App 项目结构约定：
- App/ 入口层
- Presentation/ 视图层
- Domain/ 业务层
- Data/ 数据层
- Core/ 基础设施
- Resources/ 资源

运行以下命令以进行应用的构建检查：
```bash
xcodebuild -scheme YourProjectName -sdk iphonesimulator build
```


查看有哪些可用的设备
```bash
xcrun simctl list devices available
```