use crate::constants::{
    ALIAS_EXISTS_SECTIONS, ALIAS_PATH_SECTIONS, ALL_SECTIONS, APP_NAME, AUTHOR, CONFIG_FILE,
    DATA_DIR, DATA_PATH_ENV, DEFAULT_SEARCH_ENGINE, EMAIL, REPORT_DEFAULT_FILE, REPORT_DIR,
    SCRIPTS_DIR, VERSION, config_key, section,
};
use fs2::FileExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
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
        if let Ok(path) = std::env::var(DATA_PATH_ENV) {
            return PathBuf::from(path);
        }
        // 默认路径: ~/.jdata/
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(DATA_DIR)
    }

    /// 获取配置文件路径: ~/.jdata/config.yaml
    fn config_path() -> PathBuf {
        Self::data_dir().join(CONFIG_FILE)
    }

    /// 获取脚本存储目录: ~/.jdata/scripts/
    pub fn scripts_dir() -> PathBuf {
        let dir = Self::data_dir().join(SCRIPTS_DIR);
        // 确保目录存在
        let _ = fs::create_dir_all(&dir);
        dir
    }

    /// 获取日报目录: ~/.jdata/report/
    pub fn report_dir() -> PathBuf {
        let dir = Self::data_dir().join(REPORT_DIR);
        let _ = fs::create_dir_all(&dir);
        dir
    }

    /// 获取日报文件路径（优先使用用户配置，否则使用默认路径 ~/.jdata/report/week_report.md）
    pub fn report_file_path(&self) -> PathBuf {
        if let Some(custom_path) = self.get_property(section::REPORT, config_key::WEEK_REPORT)
            && !custom_path.is_empty()
        {
            return Self::expand_tilde(custom_path);
        }
        Self::report_dir().join(REPORT_DEFAULT_FILE)
    }

    /// 展开路径中的 ~ 为用户主目录
    fn expand_tilde(path: &str) -> PathBuf {
        if path.starts_with('~')
            && let Some(home) = dirs::home_dir()
        {
            if path == "~" {
                return home;
            } else if let Some(stripped) = path.strip_prefix("~/") {
                return home.join(stripped);
            }
        }
        PathBuf::from(path)
    }

    /// 从配置文件加载
    pub fn load() -> Self {
        let path = Self::config_path();
        if !path.exists() {
            // 配置文件不存在，创建默认配置
            let config = Self::default_config();
            eprintln!("[INFO] 创建默认配置文件: {:?}", path);
            config.save_direct();
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

    /// 保存配置到文件（直接写入，不加锁，仅用于初始化时创建默认配置）
    fn save_direct(&self) {
        let path = Self::config_path();

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

    /// 并发安全保存：用文件锁保护整个 load → modify → write 临界区
    ///
    /// 流程：
    ///   1. 打开（或创建）配置文件，获取独占锁（阻塞直到获得）
    ///   2. 读取文件系统最新内容（锁内读，保证一致性）
    ///   3. 将 self 的修改应用到最新内容上
    ///   4. 写回磁盘
    ///   5. 释放锁（fd drop 时自动释放）
    ///   6. 更新 self 与磁盘保持一致
    pub fn save(&mut self) {
        let path = Self::config_path();

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap_or_else(|e| {
                eprintln!("[ERROR] 创建配置目录失败: {}", e);
            });
        }

        // 1. 打开文件并获取独占锁
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .unwrap_or_else(|e| {
                eprintln!("[ERROR] 打开配置文件失败: {}", e);
                panic!("无法打开配置文件");
            });

        lock_file.lock_exclusive().unwrap_or_else(|e| {
            eprintln!("[ERROR] 获取文件锁失败: {}", e);
        });

        // 2. 锁内读取最新内容
        let content = fs::read_to_string(&path).unwrap_or_default();
        let mut fresh: YamlConfig = serde_yaml::from_str(&content).unwrap_or_default();

        // 3. 将 self 的所有 section 应用到 fresh（self 优先）
        fresh.path = self.path.clone();
        fresh.inner_url = self.inner_url.clone();
        fresh.outer_url = self.outer_url.clone();
        fresh.editor = self.editor.clone();
        fresh.browser = self.browser.clone();
        fresh.vpn = self.vpn.clone();
        fresh.script = self.script.clone();
        fresh.version = self.version.clone();
        fresh.setting = self.setting.clone();
        fresh.log = self.log.clone();
        fresh.report = self.report.clone();
        // extra 保留 fresh 的值（不覆盖其他进程写入的未知字段）

        // 4. 序列化并写回磁盘
        let output = serde_yaml::to_string(&fresh).unwrap_or_else(|e| {
            eprintln!("[ERROR] 序列化配置失败: {}", e);
            String::new()
        });

        fs::write(&path, output).unwrap_or_else(|e| {
            eprintln!("[ERROR] 保存配置文件失败: {}, 路径: {:?}", e, path);
        });

        // 5. lock_file drop 时自动释放锁

        // 6. 同步 extra 回内存
        self.extra = fresh.extra;
    }

    /// 创建默认配置
    fn default_config() -> Self {
        let mut config = Self::default();

        // 版本信息
        config.version.insert("name".into(), APP_NAME.into());
        config.version.insert("version".into(), VERSION.into());
        config.version.insert("author".into(), AUTHOR.into());
        config.version.insert("email".into(), EMAIL.into());

        // 日志模式
        config
            .log
            .insert(config_key::MODE.into(), config_key::CONCISE.into());

        // 默认搜索引擎
        config.setting.insert(
            config_key::SEARCH_ENGINE.into(),
            DEFAULT_SEARCH_ENGINE.into(),
        );

        config
    }

    /// 是否是 verbose 模式
    pub fn is_verbose(&self) -> bool {
        self.log
            .get(config_key::MODE)
            .is_some_and(|m| m == config_key::VERBOSE)
    }

    // ========== 根据 section 名称获取对应的 map ==========

    /// 获取指定 section 的不可变引用
    pub fn get_section(&self, s: &str) -> Option<&BTreeMap<String, String>> {
        match s {
            section::PATH => Some(&self.path),
            section::INNER_URL => Some(&self.inner_url),
            section::OUTER_URL => Some(&self.outer_url),
            section::EDITOR => Some(&self.editor),
            section::BROWSER => Some(&self.browser),
            section::VPN => Some(&self.vpn),
            section::SCRIPT => Some(&self.script),
            section::VERSION => Some(&self.version),
            section::SETTING => Some(&self.setting),
            section::LOG => Some(&self.log),
            section::REPORT => Some(&self.report),
            _ => None,
        }
    }

    /// 获取指定 section 的可变引用
    pub fn get_section_mut(&mut self, s: &str) -> Option<&mut BTreeMap<String, String>> {
        match s {
            section::PATH => Some(&mut self.path),
            section::INNER_URL => Some(&mut self.inner_url),
            section::OUTER_URL => Some(&mut self.outer_url),
            section::EDITOR => Some(&mut self.editor),
            section::BROWSER => Some(&mut self.browser),
            section::VPN => Some(&mut self.vpn),
            section::SCRIPT => Some(&mut self.script),
            section::VERSION => Some(&mut self.version),
            section::SETTING => Some(&mut self.setting),
            section::LOG => Some(&mut self.log),
            section::REPORT => Some(&mut self.report),
            _ => None,
        }
    }

    /// 检查某个 section 中是否包含指定的 key
    pub fn contains(&self, section: &str, key: &str) -> bool {
        self.get_section(section)
            .is_some_and(|m| m.contains_key(key))
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
        if let Some(map) = self.get_section_mut(section)
            && let Some(value) = map.remove(old_key)
        {
            map.insert(new_key.to_string(), value);
            self.save();
        }
    }

    /// 获取所有已知的 section 名称
    pub fn all_section_names(&self) -> &'static [&'static str] {
        ALL_SECTIONS
    }

    /// 判断别名是否存在于任何 section 中（用于 open 命令判断）
    pub fn alias_exists(&self, alias: &str) -> bool {
        ALIAS_EXISTS_SECTIONS
            .iter()
            .any(|s| self.contains(s, alias))
    }

    /// 根据别名获取路径（依次从 path、inner_url、outer_url 中查找）
    pub fn get_path_by_alias(&self, alias: &str) -> Option<&String> {
        ALIAS_PATH_SECTIONS
            .iter()
            .find_map(|s| self.get_property(s, alias))
    }

    /// 收集所有别名路径，用于注入脚本执行时的环境变量
    /// 返回 Vec<(env_key, value)>，env_key 格式为 J_<ALIAS_UPPER>
    /// 别名中的 `-` 会转换为 `_`，且全部大写
    /// 覆盖 section: path, inner_url, outer_url, script
    pub fn collect_alias_envs(&self) -> Vec<(String, String)> {
        let sections = &[
            section::PATH,
            section::INNER_URL,
            section::OUTER_URL,
            section::SCRIPT,
        ];
        let mut envs = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for &sec in sections {
            if let Some(map) = self.get_section(sec) {
                for (alias, value) in map {
                    // 将别名转为环境变量名: J_<ALIAS_UPPER>，`-` 转 `_`
                    let env_key = format!("J_{}", alias.replace('-', "_").to_uppercase());
                    // 同名别名只取优先级高的（path > inner_url > outer_url > script）
                    if seen.insert(env_key.clone()) {
                        envs.push((env_key, value.clone()));
                    }
                }
            }
        }

        envs
    }
}
