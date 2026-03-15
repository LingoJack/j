# React + TailwindCSS 前端规范

## 项目结构

```
frontend/
├── public/
├── src/
│   ├── api/                # API 调用层（初始阶段返回 Mock）
│   │   ├── client.ts       # 统一 fetch 封装
│   │   ├── user.ts         # 用户模块 API
│   │   ├── product.ts      # 商品模块 API（按模块拆分）
│   │   └── ...
│   ├── mocks/              # Mock 数据
│   │   ├── user.ts
│   │   ├── product.ts
│   │   └── ...
│   ├── components/         # 通用组件
│   │   ├── Layout.tsx      # 页面布局（导航栏 + 侧边栏 + 内容区）
│   │   ├── Navbar.tsx
│   │   ├── Sidebar.tsx
│   │   ├── Table.tsx       # 通用表格
│   │   ├── Modal.tsx       # 通用弹窗
│   │   └── Loading.tsx
│   ├── pages/              # 页面组件（按模块组织）
│   │   ├── user/
│   │   │   ├── LoginPage.tsx
│   │   │   ├── RegisterPage.tsx
│   │   │   └── ProfilePage.tsx
│   │   ├── product/
│   │   │   ├── ProductListPage.tsx
│   │   │   └── ProductDetailPage.tsx
│   │   └── ...
│   ├── hooks/              # 自定义 hooks
│   │   └── useAuth.ts
│   ├── store/              # 状态管理（用 React Context 或 zustand）
│   │   └── auth.tsx
│   ├── router.tsx          # 路由配置
│   ├── App.tsx
│   ├── main.tsx
│   └── index.css           # TailwindCSS 入口
├── index.html
├── package.json
├── vite.config.ts
└── tailwind.config.js
```

## 初始化命令

```bash
npm create vite@latest frontend -- --template react-ts
cd frontend
npm install
npm install -D tailwindcss @tailwindcss/vite
npm install react-router-dom
```

vite.config.ts 中添加 TailwindCSS 插件：

```ts
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [react(), tailwindcss()],
})
```

index.css 顶部：

```css
@import "tailwindcss";
```

## Mock 数据模式

初始阶段，API 层直接返回 Mock 数据。关键点：**函数签名与真实 API 一致**，这样后续替换 Mock 只需改实现，不需改调用方。

```ts
// src/mocks/product.ts
export const mockProducts = [
  { id: 1, name: "iPhone 16", price: 7999, category: "手机", image: "https://placehold.co/300x300", stock: 100 },
  { id: 2, name: "MacBook Pro", price: 14999, category: "电脑", image: "https://placehold.co/300x300", stock: 50 },
];

// src/api/product.ts
import { mockProducts } from "../mocks/product";

const USE_MOCK = import.meta.env.VITE_USE_MOCK !== "false";
const API_BASE = import.meta.env.VITE_API_BASE_URL || "http://localhost:8080/api";

export async function getProducts() {
  if (USE_MOCK) return { code: 0, data: mockProducts, message: "ok" };
  const res = await fetch(`${API_BASE}/products`);
  return res.json();
}

export async function getProduct(id: number) {
  if (USE_MOCK) return { code: 0, data: mockProducts.find(p => p.id === id), message: "ok" };
  const res = await fetch(`${API_BASE}/products/${id}`);
  return res.json();
}
```

## 环境变量

```env
# .env
VITE_USE_MOCK=true
VITE_API_BASE_URL=http://localhost:8080/api
```

Mock 阶段 `VITE_USE_MOCK=true`，联调时改为 `false`。

## 页面组件模式

```tsx
// src/pages/product/ProductListPage.tsx
import { useEffect, useState } from "react";
import { getProducts } from "../../api/product";

export default function ProductListPage() {
  const [products, setProducts] = useState([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    getProducts().then(res => {
      if (res.code === 0) setProducts(res.data);
      setLoading(false);
    });
  }, []);

  if (loading) return <div className="p-8 text-center">加载中...</div>;

  return (
    <div className="p-6">
      <h1 className="text-2xl font-bold mb-6">商品列表</h1>
      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        {products.map(p => (
          <div key={p.id} className="border rounded-lg p-4 hover:shadow-lg transition">
            <img src={p.image} alt={p.name} className="w-full h-48 object-cover rounded" />
            <h2 className="mt-2 font-semibold">{p.name}</h2>
            <p className="text-red-500 font-bold">¥{p.price}</p>
          </div>
        ))}
      </div>
    </div>
  );
}
```

## 布局组件

```tsx
// src/components/Layout.tsx
import { Outlet } from "react-router-dom";
import Navbar from "./Navbar";
import Sidebar from "./Sidebar";

export default function Layout() {
  return (
    <div className="min-h-screen bg-gray-50">
      <Navbar />
      <div className="flex">
        <Sidebar />
        <main className="flex-1 p-6">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
```

## 路由配置

```tsx
// src/router.tsx
import { createBrowserRouter } from "react-router-dom";
import Layout from "./components/Layout";

const router = createBrowserRouter([
  {
    path: "/",
    element: <Layout />,
    children: [
      { index: true, element: <HomePage /> },
      { path: "products", element: <ProductListPage /> },
      { path: "products/:id", element: <ProductDetailPage /> },
      { path: "login", element: <LoginPage /> },
      { path: "register", element: <RegisterPage /> },
      // ...按模块追加
    ],
  },
]);
```

## 样式约定

- 使用 TailwindCSS utility class，不写自定义 CSS
- 响应式：`sm:` `md:` `lg:` 断点
- 颜色使用 Tailwind 内置色板，主色用 `blue`，强调色用 `red`（价格等）
- 间距统一用 `p-4` `p-6` `p-8`，不用奇数值
