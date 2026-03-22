SHELL := /bin/bash

.PHONY: current_dir status push pull \
        init-frontend check-frontend dev-frontend

current_dir: ## 显示当前目录信息
	@echo "当前目录: $$(pwd)"
	@echo "分支: $$(git rev-parse --abbrev-ref HEAD)"

status: current_dir ## 查看当前变更
	@git status

push: current_dir ## 提交并推送到 main
	@git add . \
	&& (git commit -m "update: $(shell date +'%Y-%m-%d %H:%M:%S')" || exit 0) \
	&& git push origin main

pull: ## 从 main 拉取最新代码
	@git pull origin main

# ============================================
# 前端
# ============================================
init-frontend: ## 初始化前端 React+TS+Tailwind (用法: make init-frontend APP=myapp)
ifndef APP
	@echo "用法: make init-frontend APP=myapp"
	@exit 1
endif
	@mkdir -p frontend
	@echo "创建 Vite + React + TypeScript 项目..."
	@cd frontend && (echo "" | npx -y create-vite@latest $(APP) --template react-ts)
	@echo "安装依赖..."
	@cd frontend/$(APP) && npm install --prefer-offline
	@echo "安装 Tailwind CSS v4..."
	@cd frontend/$(APP) && npm install -D tailwindcss @tailwindcss/vite
	@echo "配置 vite.config.ts..."
	@printf '%s\n' \
		'import { defineConfig } from "vite"' \
		'import react from "@vitejs/plugin-react"' \
		'import tailwindcss from "@tailwindcss/vite"' \
		'' \
		'export default defineConfig({' \
		'  plugins: [' \
		'    react(),' \
		'    tailwindcss(),' \
		'  ],' \
		'})' > frontend/$(APP)/vite.config.ts
	@echo "配置 index.css..."
	@echo '@import "tailwindcss";' > frontend/$(APP)/src/index.css
	@rm -f frontend/$(APP)/src/App.css
	@echo "配置 App.tsx..."
	@printf '%s\n' \
		'function App() {' \
		'  return (' \
		'    <div className="min-h-screen flex items-center justify-center bg-gray-100">' \
		'      <h1 className="text-4xl font-bold text-blue-600">Hello, World!</h1>' \
		'    </div>' \
		'  )' \
		'}' \
		'' \
		'export default App' > frontend/$(APP)/src/App.tsx
	@echo "前端初始化完成: frontend/$(APP)"

check-frontend:
	@cd frontend && npm install -y && (npm run build || exit 1)

dev-frontend: check-frontend
	@cd frontend && npm run dev
