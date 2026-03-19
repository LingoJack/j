project_name = $1
mkdir -p frontend/
cd frontend
echo "" | npx -y create-vite project_name --template react-ts
cd project_name/
npm install