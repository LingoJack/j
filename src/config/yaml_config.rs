use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

/// YAML 配置文件的完整结构
/// 使用 BTreeMap 保持键的有序性，与 Java 版的 LinkedHashMap 行为一致
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct YamlConfig {
    #[serde(default)]
    pub path: BTreeMap<String, String>,

    #[serde(default)]
    pub inner_url: BTreeMap<String, String>,

    #[serde(default)]
    pub outer_url: BTreeMap<String, String>,

    #[serde(default)]
    pub editor: BTreeMap<String, String>,

    #[serde(default)]
    pub browser: BTreeMap<String, String>,

    #[serde(default)]
    pub vpn: BTreeMap<String, String>,

    #[serde(default)]
    pub script: BTreeMap<String, String>,

    #[serde(default)]
    pub version: BTreeMap<String, String>,

    #[serde(default)]
    pub setting: BTreeMap<String, String>,

    #[serde(default)]
    pub log: BTreeMap<String, String>,

    #[serde(default)]
    pub report: BTreeMap<String, String>,

    /// 捕获未知的顶级键，保证不丢失任何配置
    #[serde(flatten)]
    pub extra: BTreeMap<String, serde_yaml::Value>,
}

impl YamlConfig {
    /// 获取数据根目录: ~/.jdata/
    pub fn data_dir() -> PathBuf {
        // 优先使用环境变量指定的数据路径
        if let Ok(path) = std::env::var("J_DATA_PATH") {
            return PathBuf::from(path);
        }
        // 默认路径: ~/.jdata/
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".jdata")
    }

    /// 获取配置文件路径: ~/.jdata/config.yaml
    fn config_path() -> PathBuf {
        Self::data_dir().join("config.yaml")
    }

    /// 获取脚本存储目录: ~/.jdata/scripts/
    pub fn scripts_dir() -> PathBuf {
        let dir = Self::data_dir().join("scripts");
        // 确保目录存在
        let _ = fs::create_dir_all(&dir);
        dir
    }

    /// 从配置文件加载
    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() {
            // 配置文件不存在，创建默认配置
            let config = Self::default_config();
            config.save();
            return config;
        }

        let content = fs::read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("[ERROR] 读取配置文件失败: {}, 路径: {:?}", e, path);
            String::new()
        });

        serde_yaml::from_str(&content).unwrap_or_else(|e| {
            eprintln!("[ERROR] 解析配置文件失败: {}, 路径: {:?}", e, path);
            Self::default_config()
        })
    }

    /// 保存配置到文件
    pub fn save(&self) {
        let path = Self::config_path();

        // 确保目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|e| {
                eprintln!("[ERROR] 创建配置目录失败: {}", e);
            });
        }

        let content = serde_yaml::to_string(self).unwrap_or_else(|e| {
            eprintln!("[ERROR] 序列化配置失败: {}", e);
            String::new()
        });

        fs::write(&path, content).unwrap_or_else(|e| {
            eprintln!("[ERROR] 保存配置文件失败: {}, 路径: {:?}", e, path);
        });
    }

    /// 创建默认配置
    fn default_config() -> Self {
        let mut config = Self::default();

        // 版本信息
        config.version.insert("name".into(), "work-copilot".into());
        config.version.insert("version".into(), "11.0.0".into());
        config.version.insert("author".into(), "lingojack".into());
        config
            .version
            .insert("email".into(), "lingojack@qq.com".into());

        // 日志模式
        config.log.insert("mode".into(), "concise".into());

        // 默认搜索引擎
        config.setting.insert("search-engine".into(), "bing".into());

        config
    }

    /// 是否是 verbose 模式
    pub fn is_verbose(&self) -> bool {
        self.log.get("mode").map_or(false, |m| m == "verbose")
    }

    // ========== 根据 section 名称获取对应的 map ==========

    /// 获取指定 section 的不可变引用
    pub fn get_section(&self, section: &str) -> Option<&BTreeMap<String, String>> {
        match section {
            "path" => Some(&self.path),
            "inner_url" => Some(&self.inner_url),
            "outer_url" => Some(&self.outer_url),
            "editor" => Some(&self.editor),
            "browser" => Some(&self.browser),
            "vpn" => Some(&self.vpn),
            "script" => Some(&self.script),
            "version" => Some(&self.version),
            "setting" => Some(&self.setting),
            "log" => Some(&self.log),
            "report" => Some(&self.report),
            _ => None,
        }
    }

    /// 获取指定 section 的可变引用
    pub fn get_section_mut(&mut self, section: &str) -> Option<&mut BTreeMap<String, String>> {
        match section {
            "path" => Some(&mut self.path),
            "inner_url" => Some(&mut self.inner_url),
            "outer_url" => Some(&mut self.outer_url),
            "editor" => Some(&mut self.editor),
            "browser" => Some(&mut self.browser),
            "vpn" => Some(&mut self.vpn),
            "script" => Some(&mut self.script),
            "version" => Some(&mut self.version),
            "setting" => Some(&mut self.setting),
            "log" => Some(&mut self.log),
            "report" => Some(&mut self.report),
            _ => None,
        }
    }

    /// 检查某个 section 中是否包含指定的 key
    pub fn contains(&self, section: &str, key: &str) -> bool {
        self.get_section(section)
            .map_or(false, |m| m.contains_key(key))
    }

    /// 获取某个 section 中指定 key 的值
    pub fn get_property(&self, section: &str, key: &str) -> Option<&String> {
        self.get_section(section).and_then(|m| m.get(key))
    }

    /// 设置某个 section 中的键值对并保存
    pub fn set_property(&mut self, section: &str, key: &str, value: &str) {
        if let Some(map) = self.get_section_mut(section) {
            map.insert(key.to_string(), value.to_string());
            self.save();
        }
    }

    /// 删除某个 section 中的键并保存
    pub fn remove_property(&mut self, section: &str, key: &str) {
        if let Some(map) = self.get_section_mut(section) {
            map.remove(key);
            self.save();
        }
    }

    /// 重命名某个 section 中的键
    pub fn rename_property(&mut self, section: &str, old_key: &str, new_key: &str) {
        if let Some(map) = self.get_section_mut(section) {
            if let Some(value) = map.remove(old_key) {
                map.insert(new_key.to_string(), value);
                self.save();
            }
        }
    }

    /// 获取所有已知的 section 名称
    pub fn all_section_names(&self) -> Vec<&str> {
        vec![
            "path",
            "inner_url",
            "outer_url",
            "editor",
            "browser",
            "vpn",
            "script",
            "version",
            "setting",
            "log",
            "report",
        ]
    }

    /// 判断别名是否存在于任何 section 中（用于 open 命令判断）
    pub fn alias_exists(&self, alias: &str) -> bool {
        self.contains("path", alias)
            || self.contains("inner_url", alias)
            || self.contains("outer_url", alias)
            || self.contains("script", alias)
            || self.contains("browser", alias)
            || self.contains("editor", alias)
            || self.contains("vpn", alias)
    }

    /// 根据别名获取路径（依次从 path、inner_url、outer_url 中查找）
    pub fn get_path_by_alias(&self, alias: &str) -> Option<&String> {
        self.get_property("path", alias)
            .or_else(|| self.get_property("inner_url", alias))
            .or_else(|| self.get_property("outer_url", alias))
    }
}
