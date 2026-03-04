package tool

import (
	"encoding/json"
	"errors"
	"fmt"

	"github.com/invopop/jsonschema"
)

// SchemaOption 定义JSON Schema生成选项（使用位运算）
//
// 通过位运算可以灵活组合多个选项，例如：
//   - SchemaOptionAnonymous | SchemaOptionExpandStruct
//   - SchemaOptionCompact | SchemaOptionNoReference
//
// 使用示例:
//
//	opts := SchemaOptionAnonymous | SchemaOptionExpandStruct | SchemaOptionNoReference
//	schema, err := StructToJSONSchema(&User{}, opts)
type SchemaOption uint32

const (
	// SchemaOptionNone 默认选项，不做任何特殊配置
	SchemaOptionNone SchemaOption = 0

	// SchemaOptionAnonymous 不生成$id字段
	// 适用于不需要Schema标识符的场景
	SchemaOptionAnonymous SchemaOption = 1 << 0 // 1

	// SchemaOptionExpandStruct 展开嵌套结构体而不是使用$ref引用
	// 适用于需要完整Schema定义的场景
	SchemaOptionExpandStruct SchemaOption = 1 << 1 // 2

	// SchemaOptionNoReference 不使用$ref引用
	// 适用于需要内联所有定义的场景
	SchemaOptionNoReference SchemaOption = 1 << 2 // 4

	// SchemaOptionNoAdditionalProperties 不允许额外属性
	// 适用于需要严格验证的场景
	SchemaOptionNoAdditionalProperties SchemaOption = 1 << 3 // 8

	// SchemaOptionAllRequired 忽略jsonschema标签中的required信息
	// 默认会从标签读取required，只有在不需要从标签读取时才使用此选项，使用此标签时，所有字段为requeired
	SchemaOptionAllRequired SchemaOption = 1 << 4 // 16

	// SchemaOptionCompact 生成紧凑格式的JSON（不带缩进）
	// 适用于网络传输或存储场景
	SchemaOptionCompact SchemaOption = 1 << 5 // 32

	// SchemaOptionAllowNullValues 允许字段为null值
	// 适用于需要支持null的场景
	SchemaOptionAllowNullValues SchemaOption = 1 << 6 // 64
)

// Has 检查是否包含指定选项
//
// 使用示例:
//
//	opts := SchemaOptionAnonymous | SchemaOptionCompact
//	if opts.Has(SchemaOptionCompact) {
//	    fmt.Println("使用紧凑格式")
//	}
func (o SchemaOption) Has(option SchemaOption) bool {
	return o&option != 0
}

// StructToJSONSchema 将Go结构体转换为JSON Schema字符串
//
// 该函数使用invopop/jsonschema库通过反射机制将Go结构体转换为符合JSON Schema Draft 2020-12标准的Schema定义。
//
// 参数:
//   - structInstance: 要转换的结构体实例（必须传入指针或实例）
//   - options: 配置选项，可以通过位运算组合多个选项（可选参数，默认为SchemaOptionNone）
//
// 返回值:
//   - schemaJSON: JSON Schema的字符串表示（格式化后的JSON）
//   - err: 如果转换或序列化时发生错误，返回相应的错误信息
//
// 使用示例:
//
//	type User struct {
//	    Name  string `json:"name" jsonschema:"title=用户名,description=用户的姓名,minLength=1,maxLength=50"`
//	    Age   int    `json:"age" jsonschema:"title=年龄,description=用户的年龄,minimum=0,maximum=150"`
//	    Email string `json:"email" jsonschema:"title=邮箱,description=用户的电子邮箱,format=email"`
//	}
//
//	// 基本使用（使用默认选项）
//	schema, err := StructToJSONSchema(&User{})
//	if err != nil {
//	    log.Fatal(err)
//	}
//	fmt.Println(schema)
//
//	// 使用单个选项
//	schema, err := StructToJSONSchema(&User{}, SchemaOptionAnonymous)
//
//	// 组合多个选项
//	schema, err := StructToJSONSchema(&User{},
//	    SchemaOptionAnonymous | SchemaOptionExpandStruct | SchemaOptionNoReference)
//
//	// 生成紧凑格式
//	schema, err := StructToJSONSchema(&User{}, SchemaOptionCompact)
//
// 输出示例:
//
//	{
//	  "$schema": "https://json-schema.org/draft/2020-12/schema",
//	  "type": "object",
//	  "properties": {
//	    "name": {
//	      "type": "string",
//	      "title": "用户名",
//	      "description": "用户的姓名",
//	      "minLength": 1,
//	      "maxLength": 50
//	    },
//	    "age": {
//	      "type": "integer",
//	      "title": "年龄",
//	      "description": "用户的年龄",
//	      "minimum": 0,
//	      "maximum": 150
//	    },
//	    "email": {
//	      "type": "string",
//	      "title": "邮箱",
//	      "description": "用户的电子邮箱",
//	      "format": "email"
//	    }
//	  }
//	}
//
// 支持的jsonschema标签:
//   - title: 字段标题
//   - description: 字段描述
//   - required: 必填字段（在结构体级别使用）
//   - enum: 枚举值（例如: enum=active|inactive|pending）
//   - minLength/maxLength: 字符串长度限制
//   - minimum/maximum: 数值范围限制
//   - format: 格式验证（如email、uri、date-time等）
//   - pattern: 正则表达式验证
//   - default: 默认值
//
// 注意事项:
//   - 建议传入结构体指针以获得更准确的Schema
//   - 使用json标签定义字段名称，使用jsonschema标签定义验证规则
//   - 支持嵌套结构体、切片、映射等复杂类型
//   - 生成的Schema符合JSON Schema Draft 2020-12标准
func StructToJSONSchema(structInstance interface{}, options ...SchemaOption) (schemaJSON string, err error) {
	// 合并所有选项
	var opt = SchemaOptionNone
	if len(options) > 0 {
		for _, o := range options {
			opt |= o
		}
	}

	// 创建并配置Reflector实例
	reflector := &jsonschema.Reflector{}
	applyOptions(reflector, opt)

	// 通过反射生成JSON Schema
	schema := reflector.Reflect(structInstance)

	// 根据选项决定序列化格式
	var schemaBytes []byte
	if opt.Has(SchemaOptionCompact) {
		schemaBytes, err = json.Marshal(schema)
	} else {
		schemaBytes, err = json.MarshalIndent(schema, "", "  ")
	}

	if err != nil {
		err = errors.New(fmt.Sprintf("json schema marshal error: %v", err))
		return
	}

	schemaJSON = string(schemaBytes)
	return
}

// applyOptions 将位运算选项应用到Reflector配置
//
// 该函数是内部辅助函数，用于将SchemaOption转换为Reflector的具体配置。
//
// 参数:
//   - reflector: jsonschema.Reflector实例
//   - opt: 位运算组合的选项
func applyOptions(reflector *jsonschema.Reflector, opt SchemaOption) {
	// 默认从标签读取required信息，除非显式声明忽略
	reflector.RequiredFromJSONSchemaTags = !opt.Has(SchemaOptionAllRequired)

	if opt.Has(SchemaOptionAnonymous) {
		reflector.Anonymous = true
	}

	if opt.Has(SchemaOptionExpandStruct) {
		reflector.ExpandedStruct = true
	}

	if opt.Has(SchemaOptionNoReference) {
		reflector.DoNotReference = true
	}

	if opt.Has(SchemaOptionNoAdditionalProperties) {
		reflector.AllowAdditionalProperties = false
	}

	if opt.Has(SchemaOptionAllowNullValues) {
		reflector.AllowAdditionalProperties = true
	}
}

// StructToJSONSchemaObject 将Go结构体转换为JSON Schema对象
//
// 该函数返回jsonschema.Schema对象而不是JSON字符串，适用于需要进一步处理Schema的场景。
//
// 参数:
//   - structInstance: 要转换的结构体实例
//   - options: 配置选项，可以通过位运算组合多个选项（可选参数）
//
// 返回值:
//   - schema: jsonschema.Schema对象，可以进行进一步的操作和修改
//
// 使用示例:
//
//	type Config struct {
//	    Host string `json:"host"`
//	    Port int    `json:"port"`
//	}
//
//	// 使用默认选项
//	schema := StructToJSONSchemaObject(&Config{})
//
//	// 使用自定义选项
//	schema := StructToJSONSchemaObject(&Config{},
//	    SchemaOptionAnonymous | SchemaOptionExpandStruct)
//
//	// 可以进一步修改Schema对象
//	schema.Title = "服务器配置"
//	schema.Description = "服务器的配置信息"
//
//	// 序列化为JSON
//	schemaJSON, _ := json.MarshalIndent(schema, "", "  ")
//	fmt.Println(string(schemaJSON))
//
// 注意事项:
//   - 返回的Schema对象可以被修改和扩展
//   - 适用于需要动态调整Schema的场景
//   - 需要手动序列化为JSON字符串
func StructToJSONSchemaObject(structInstance interface{}, options ...SchemaOption) *jsonschema.Schema {
	// 合并所有选项
	var opt SchemaOption = SchemaOptionNone
	if len(options) > 0 {
		for _, o := range options {
			opt |= o
		}
	}

	// 创建并配置Reflector实例
	reflector := &jsonschema.Reflector{}
	applyOptions(reflector, opt)

	return reflector.Reflect(structInstance)
}

// MustStructToJSONSchema 将Go结构体转换为JSON Schema字符串，如果失败则panic
//
// 该函数是StructToJSONSchema的panic版本，适用于确定不会出错的场景。
//
// 参数:
//   - structInstance: 要转换的结构体实例
//   - options: 配置选项，可以通过位运算组合多个选项（可选参数）
//
// 返回值:
//   - schemaJSON: JSON Schema的字符串表示
//
// 使用示例:
//
//	type Settings struct {
//	    Theme    string `json:"theme"`
//	    Language string `json:"language"`
//	}
//
//	// 在初始化阶段使用，如果失败会panic
//	schema := MustStructToJSONSchema(&Settings{})
//	fmt.Println(schema)
//
//	// 使用自定义选项
//	schema := MustStructToJSONSchema(&Settings{},
//	    SchemaOptionAnonymous | SchemaOptionCompact)
//
// 注意事项:
//   - 仅在确定结构体定义正确且不会出错时使用
//   - 适用于程序初始化阶段的配置加载
//   - 如果转换失败会导致程序panic
func MustStructToJSONSchema(structInstance interface{}, options ...SchemaOption) (schemaJSON string) {
	schemaJSON, err := StructToJSONSchema(structInstance, options...)
	if err != nil {
		panic(err)
	}
	return schemaJSON
}

// ValidateJSONAgainstStruct 验证JSON数据是否符合结构体定义的Schema
//
// 该函数先将结构体转换为JSON Schema，然后验证给定的JSON数据是否符合该Schema。
// 注意：此函数仅生成Schema，实际验证需要配合JSON Schema验证库使用。
//
// 参数:
//   - structInstance: 结构体实例，用于生成Schema
//   - jsonData: 要验证的JSON字符串
//   - options: 配置选项，可以通过位运算组合多个选项（可选参数）
//
// 返回值:
//   - valid: 是否有效（当前实现始终返回true，需要配合验证库）
//   - schema: 生成的JSON Schema字符串
//   - err: 如果生成Schema时发生错误，返回相应的错误信息
//
// 使用示例:
//
//	type User struct {
//	    Name string `json:"name" jsonschema:"required"`
//	    Age  int    `json:"age" jsonschema:"minimum=0"`
//	}
//
//	jsonData := `{"name": "张三", "age": 25}`
//	valid, schema, err := ValidateJSONAgainstStruct(&User{}, jsonData)
//	if err != nil {
//	    log.Fatal(err)
//	}
//	fmt.Printf("Valid: %v\nSchema: %s\n", valid, schema)
//
//	// 使用自定义选项
//	valid, schema, err := ValidateJSONAgainstStruct(&User{}, jsonData,
//	    SchemaOptionNoAdditionalProperties)
//
// 注意事项:
//   - 当前实现仅生成Schema，不执行实际验证
//   - 如需完整验证功能，建议配合github.com/xeipuuv/gojsonschema等验证库使用
//   - 返回的schema可用于前端验证或文档生成
func ValidateJSONAgainstStruct(structInstance interface{}, jsonData string, options ...SchemaOption) (valid bool, schema string, err error) {
	// 生成JSON Schema
	schema, err = StructToJSONSchema(structInstance, options...)
	if err != nil {
		return
	}

	// 注意：这里仅生成Schema，实际验证需要使用专门的JSON Schema验证库
	// 例如: github.com/xeipuuv/gojsonschema
	// 这里返回true仅作为示例，实际使用时应该实现真正的验证逻辑
	valid = true

	return
}

// GetDefaultOptions 获取推荐的默认选项组合
//
// 该函数返回一组常用的选项组合，适用于大多数场景。
//
// 返回值:
//   - options: 推荐的默认选项组合
//
// 使用示例:
//
//	opts := GetDefaultOptions()
//	schema, err := StructToJSONSchema(&User{}, opts)
func GetDefaultOptions() SchemaOption {
	return SchemaOptionAnonymous | SchemaOptionExpandStruct
}

// GetCompactOptions 获取紧凑格式的选项组合
//
// 该函数返回适用于网络传输或存储的紧凑格式选项组合。
//
// 返回值:
//   - options: 紧凑格式的选项组合
//
// 使用示例:
//
//	opts := GetCompactOptions()
//	schema, err := StructToJSONSchema(&User{}, opts)
func GetCompactOptions() SchemaOption {
	return SchemaOptionCompact | SchemaOptionAnonymous | SchemaOptionNoReference
}

// GetStrictOptions 获取严格验证的选项组合
//
// 该函数返回适用于需要严格验证的选项组合。
//
// 返回值:
//   - options: 严格验证的选项组合
//
// 使用示例:
//
//	opts := GetStrictOptions()
//	schema, err := StructToJSONSchema(&User{}, opts)
func GetStrictOptions() SchemaOption {
	return SchemaOptionNoAdditionalProperties
}

// =============================================================================
// SchemaBuilder - 链式调用的 JSON Schema 构建器
// =============================================================================

// FieldMeta 字段元数据，用于存储字段的额外描述信息
type FieldMeta struct {
	Title       string      // 字段标题
	Description string      // 字段描述（支持长文本和特殊字符）
	Example     interface{} // 示例值
	Default     interface{} // 默认值
	Enum        []string    // 枚举值
	Deprecated  bool        // 是否已废弃
	ReadOnly    bool        // 是否只读
	WriteOnly   bool        // 是否只写
}

// SchemaBuilder JSON Schema 构建器，支持链式调用
//
// 使用示例:
//
//	type User struct {
//	    Name  string `json:"name"`
//	    Age   int    `json:"age"`
//	    Email string `json:"email,omitempty"`
//	}
//
//	// 使用 Builder 模式
//	schema, err := NewSchemaBuilder(&User{}).
//	    WithOptions(SchemaOptionAnonymous | SchemaOptionExpandStruct).
//	    SetTitle("用户信息").
//	    SetDescription("用户的基本信息结构").
//	    SetFieldMeta("name", FieldMeta{
//	        Title:       "用户名",
//	        Description: `用户的姓名，支持中英文。
//	注意事项：
//	1. 长度限制为 1-50 个字符
//	2. 不能包含特殊字符如 \` 反引号`,
//	    }).
//	    SetFieldMeta("age", FieldMeta{
//	        Title:       "年龄",
//	        Description: "用户年龄，范围 0-150",
//	        Example:     25,
//	    }).
//	    SetFieldMeta("email", FieldMeta{
//	        Title:       "邮箱",
//	        Description: "用户邮箱地址",
//	        Example:     "user@example.com",
//	    }).
//	    Build()
type SchemaBuilder struct {
	instance    interface{}            // 结构体实例
	options     SchemaOption           // 配置选项
	title       string                 // Schema 标题
	description string                 // Schema 描述
	fieldMetas  map[string]*FieldMeta  // 字段元数据映射
	extraProps  map[string]interface{} // 额外属性
}

// NewSchemaBuilder 创建新的 SchemaBuilder 实例
//
// 参数:
//   - structInstance: 要转换的结构体实例（建议传入指针）
//
// 返回值:
//   - *SchemaBuilder: 构建器实例，支持链式调用
//

//	// 1. 定义结构体（使用 json 标签定义字段名，jsonschema 标签定义验证规则）
//	type User struct {
//	    Name  string `json:"name" jsonschema:"required"`
//	    Age   int    `json:"age"`
//	    Email string `json:"email,omitempty"`
//	}
//
//	// 2. 创建 Builder 并链式配置
//	schema, err := NewSchemaBuilder(&User{}).
//	    WithOptions(SchemaOptionAnonymous | SchemaOptionExpandStruct | SchemaOptionNoReference).
//	    SetTitle("用户信息").
//	    SetDescription("用户的基本信息结构").
//	    SetFieldMeta("name", FieldMeta{
//	        Title:       "用户名",
//	        Description: "用户的姓名，长度 1-50 个字符",
//	    }).
//	    SetFieldMeta("age", FieldMeta{
//	        Title:       "年龄",
//	        Description: "用户年龄，范围 0-150",
//	        Example:     25,
//	        Default:     0,
//	    }).
//	    SetFieldMeta("email", FieldMeta{
//	        Title:       "邮箱",
//	        Description: "用户邮箱地址",
//	        Example:     "user@example.com",
//	    }).
//	    Build()
//
// 快捷设置字段描述（不需要构造 FieldMeta）:
//
//	schema, err := NewSchemaBuilder(&User{}).
//	    WithOptions(SchemaOptionAnonymous | SchemaOptionExpandStruct).
//	    SetFieldTitle("name", "用户名").
//	    SetFieldDescription("name", "用户的姓名").
//	    SetFieldExample("name", "张三").
//	    SetFieldDefault("age", 18).
//	    SetFieldEnum("status", []string{"active", "inactive"}).
//	    SetFieldDeprecated("old_field").
//	    Build()
//
// 获取 Schema 对象（而非 JSON 字符串，可进一步修改）:
//
//	schemaObj := NewSchemaBuilder(&User{}).
//	    WithOptions(SchemaOptionAnonymous | SchemaOptionExpandStruct).
//	    BuildObject()
//
// 使用 MustBuild（失败时 panic，适合初始化阶段）:
//
//	schema := NewSchemaBuilder(&User{}).
//	    WithOptions(SchemaOptionAnonymous | SchemaOptionExpandStruct).
//	    MustBuild()
//
// 使用全局注册表共享字段描述:
//
//	registry := NewFieldDescRegistry()
//	registry.Register("User", "name", FieldMeta{Title: "用户名", Description: "..."})
//	registry.Register("User", "age", FieldMeta{Title: "年龄", Description: "..."})
//
//	schema, err := NewSchemaBuilder(&User{}).
//	    WithOptions(GetDefaultOptions()).
//	    ApplyRegistry(registry, "User").
//	    Build()
//
// 常用选项组合:
//   - GetDefaultOptions()  → SchemaOptionAnonymous | SchemaOptionExpandStruct（推荐默认使用）
//   - GetCompactOptions()  → SchemaOptionCompact | SchemaOptionAnonymous | SchemaOptionNoReference（适合网络传输）
//   - GetStrictOptions()   → SchemaOptionNoAdditionalProperties（严格验证，不允许额外属性）
//
// 也可使用快捷函数 QuickSchema / QuickSchemaCompact 来简化调用:
//
//	schema, err := QuickSchema(&User{}, map[string]FieldMeta{
//	    "name": {Title: "用户名", Description: "用户的姓名"},
//	})
func NewSchemaBuilder(structInstance interface{}) *SchemaBuilder {
	return &SchemaBuilder{
		instance:   structInstance,
		options:    SchemaOptionNone,
		fieldMetas: make(map[string]*FieldMeta),
		extraProps: make(map[string]interface{}),
	}
}

// WithOptions 设置 Schema 生成选项
//
// 参数:
//   - options: 配置选项，支持位运算组合
//
// 返回值:
//   - *SchemaBuilder: 返回自身以支持链式调用
func (b *SchemaBuilder) WithOptions(options SchemaOption) *SchemaBuilder {
	b.options = options
	return b
}

// SetTitle 设置 Schema 标题
//
// 参数:
//   - title: Schema 标题
//
// 返回值:
//   - *SchemaBuilder: 返回自身以支持链式调用
func (b *SchemaBuilder) SetTitle(title string) *SchemaBuilder {
	b.title = title
	return b
}

// SetDescription 设置 Schema 描述（支持长文本和特殊字符）
//
// 参数:
//   - description: Schema 描述
//
// 返回值:
//   - *SchemaBuilder: 返回自身以支持链式调用
func (b *SchemaBuilder) SetDescription(description string) *SchemaBuilder {
	b.description = description
	return b
}

// SetFieldMeta 设置字段的元数据
//
// 参数:
//   - fieldName: 字段的 JSON 名称（使用 json tag 中定义的名称）
//   - meta: 字段元数据
//
// 返回值:
//   - *SchemaBuilder: 返回自身以支持链式调用
//
// 使用示例:
//
//	builder.SetFieldMeta("name", FieldMeta{
//	    Title:       "用户名",
//	    Description: `这是一段很长的描述，
//	可以包含换行符和特殊字符如反引号 \``,
//	    Example:     "张三",
//	})
func (b *SchemaBuilder) SetFieldMeta(fieldName string, meta FieldMeta) *SchemaBuilder {
	b.fieldMetas[fieldName] = &meta
	return b
}

// SetFieldTitle 快捷方法：设置字段标题
//
// 参数:
//   - fieldName: 字段的 JSON 名称
//   - title: 字段标题
//
// 返回值:
//   - *SchemaBuilder: 返回自身以支持链式调用
func (b *SchemaBuilder) SetFieldTitle(fieldName, title string) *SchemaBuilder {
	if _, ok := b.fieldMetas[fieldName]; !ok {
		b.fieldMetas[fieldName] = &FieldMeta{}
	}
	b.fieldMetas[fieldName].Title = title
	return b
}

// SetFieldDescription 快捷方法：设置字段描述
//
// 参数:
//   - fieldName: 字段的 JSON 名称
//   - description: 字段描述（支持长文本和特殊字符）
//
// 返回值:
//   - *SchemaBuilder: 返回自身以支持链式调用
//
// 使用示例:
//
//	builder.SetFieldDescription("config", `配置说明：
//	这是一段很长的描述文本，可以包含：
//	- 换行符
//	- 特殊字符如反引号 \`
//	- Markdown 格式
//	- 代码示例等`)
func (b *SchemaBuilder) SetFieldDescription(fieldName, description string) *SchemaBuilder {
	if _, ok := b.fieldMetas[fieldName]; !ok {
		b.fieldMetas[fieldName] = &FieldMeta{}
	}
	b.fieldMetas[fieldName].Description = description
	return b
}

// SetFieldExample 快捷方法：设置字段示例值
//
// 参数:
//   - fieldName: 字段的 JSON 名称
//   - example: 示例值
//
// 返回值:
//   - *SchemaBuilder: 返回自身以支持链式调用
func (b *SchemaBuilder) SetFieldExample(fieldName string, example interface{}) *SchemaBuilder {
	if _, ok := b.fieldMetas[fieldName]; !ok {
		b.fieldMetas[fieldName] = &FieldMeta{}
	}
	b.fieldMetas[fieldName].Example = example
	return b
}

// SetFieldDefault 快捷方法：设置字段默认值
//
// 参数:
//   - fieldName: 字段的 JSON 名称
//   - defaultValue: 默认值
//
// 返回值:
//   - *SchemaBuilder: 返回自身以支持链式调用
func (b *SchemaBuilder) SetFieldDefault(fieldName string, defaultValue interface{}) *SchemaBuilder {
	if _, ok := b.fieldMetas[fieldName]; !ok {
		b.fieldMetas[fieldName] = &FieldMeta{}
	}
	b.fieldMetas[fieldName].Default = defaultValue
	return b
}

// SetFieldEnum 快捷方法：设置字段枚举值
//
// 参数:
//   - fieldName: 字段的 JSON 名称
//   - enum: 枚举值列表
//
// 返回值:
//   - *SchemaBuilder: 返回自身以支持链式调用
func (b *SchemaBuilder) SetFieldEnum(fieldName string, enum []string) *SchemaBuilder {
	if _, ok := b.fieldMetas[fieldName]; !ok {
		b.fieldMetas[fieldName] = &FieldMeta{}
	}
	b.fieldMetas[fieldName].Enum = enum
	return b
}

// SetFieldDeprecated 快捷方法：标记字段为已废弃
//
// 参数:
//   - fieldName: 字段的 JSON 名称
//
// 返回值:
//   - *SchemaBuilder: 返回自身以支持链式调用
func (b *SchemaBuilder) SetFieldDeprecated(fieldName string) *SchemaBuilder {
	if _, ok := b.fieldMetas[fieldName]; !ok {
		b.fieldMetas[fieldName] = &FieldMeta{}
	}
	b.fieldMetas[fieldName].Deprecated = true
	return b
}

// SetExtraProperty 设置 Schema 的额外属性
//
// 参数:
//   - key: 属性键名
//   - value: 属性值
//
// 返回值:
//   - *SchemaBuilder: 返回自身以支持链式调用
func (b *SchemaBuilder) SetExtraProperty(key string, value interface{}) *SchemaBuilder {
	b.extraProps[key] = value
	return b
}

// Build 构建并返回 JSON Schema 字符串
//
// 返回值:
//   - schemaJSON: JSON Schema 的字符串表示
//   - err: 如果构建过程中发生错误，返回相应的错误信息
//
// 使用示例:
//
//	schema, err := builder.Build()
//	if err != nil {
//	    log.Fatal(err)
//	}
//	fmt.Println(schema)
func (b *SchemaBuilder) Build() (schemaJSON string, err error) {
	schema := b.BuildObject()

	// 根据选项决定序列化格式
	var schemaBytes []byte
	if b.options.Has(SchemaOptionCompact) {
		schemaBytes, err = json.Marshal(schema)
	} else {
		schemaBytes, err = json.MarshalIndent(schema, "", "  ")
	}

	if err != nil {
		err = errors.New(fmt.Sprintf("json schema marshal error: %v", err))
		return
	}

	schemaJSON = string(schemaBytes)
	return
}

// BuildObject 构建并返回 JSON Schema 对象
//
// 返回值:
//   - *jsonschema.Schema: JSON Schema 对象，可以进一步修改
//
// 使用示例:
//
//	schema := builder.BuildObject()
//	// 进一步修改 schema
//	schema.Title = "自定义标题"
func (b *SchemaBuilder) BuildObject() *jsonschema.Schema {
	// 创建并配置 Reflector 实例
	reflector := &jsonschema.Reflector{}
	applyOptions(reflector, b.options)

	// 通过反射生成 JSON Schema
	schema := reflector.Reflect(b.instance)

	// 应用 Schema 级别的元数据
	if b.title != "" {
		schema.Title = b.title
	}
	if b.description != "" {
		schema.Description = b.description
	}

	// 应用字段级别的元数据
	b.applyFieldMetas(schema)

	// 应用额外属性
	for key, value := range b.extraProps {
		schema.Extras[key] = value
	}

	return schema
}

// MustBuild 构建并返回 JSON Schema 字符串，如果失败则 panic
//
// 返回值:
//   - schemaJSON: JSON Schema 的字符串表示
//
// 使用示例:
//
//	schema := builder.MustBuild()
func (b *SchemaBuilder) MustBuild() string {
	schemaJSON, err := b.Build()
	if err != nil {
		panic(err)
	}
	return schemaJSON
}

// applyFieldMetas 将字段元数据应用到 Schema 中
func (b *SchemaBuilder) applyFieldMetas(schema *jsonschema.Schema) {
	if schema.Properties == nil {
		return
	}

	for pair := schema.Properties.Oldest(); pair != nil; pair = pair.Next() {
		fieldName := pair.Key
		fieldSchema := pair.Value

		if meta, ok := b.fieldMetas[fieldName]; ok {
			applyMetaToSchema(fieldSchema, meta)
		}

		// 递归处理嵌套的 object 类型
		if fieldSchema.Properties != nil {
			b.applyFieldMetas(fieldSchema)
		}
	}
}

// applyMetaToSchema 将 FieldMeta 应用到 Schema 属性
func applyMetaToSchema(schema *jsonschema.Schema, meta *FieldMeta) {
	if meta.Title != "" {
		schema.Title = meta.Title
	}
	if meta.Description != "" {
		schema.Description = meta.Description
	}
	if meta.Example != nil {
		schema.Examples = []interface{}{meta.Example}
	}
	if meta.Default != nil {
		schema.Default = meta.Default
	}
	if len(meta.Enum) > 0 {
		enumValues := make([]interface{}, len(meta.Enum))
		for i, v := range meta.Enum {
			enumValues[i] = v
		}
		schema.Enum = enumValues
	}
	if meta.Deprecated {
		schema.Deprecated = true
	}
	if meta.ReadOnly {
		schema.ReadOnly = true
	}
	if meta.WriteOnly {
		schema.WriteOnly = true
	}
}

// =============================================================================
// 全局字段描述注册表 - 适用于跨多个 Schema 共享描述
// =============================================================================

// FieldDescRegistry 全局字段描述注册表
//
// 适用于以下场景：
// 1. 多个 Schema 共享相同的字段描述
// 2. 字段描述需要动态生成或从配置文件加载
// 3. 需要集中管理所有字段描述
//
// 使用示例:
//
//	// 全局注册字段描述
//	registry := NewFieldDescRegistry()
//	registry.Register("User", "name", FieldMeta{
//	    Title:       "用户名",
//	    Description: "很长的描述...",
//	})
//
//	// 在构建时应用
//	schema, _ := NewSchemaBuilder(&User{}).
//	    ApplyRegistry(registry, "User").
//	    Build()
type FieldDescRegistry struct {
	registry map[string]map[string]*FieldMeta // typeName -> fieldName -> FieldMeta
}

// NewFieldDescRegistry 创建新的字段描述注册表
func NewFieldDescRegistry() *FieldDescRegistry {
	return &FieldDescRegistry{
		registry: make(map[string]map[string]*FieldMeta),
	}
}

// Register 注册字段描述
//
// 参数:
//   - typeName: 类型名称（用于区分不同的结构体）
//   - fieldName: 字段的 JSON 名称
//   - meta: 字段元数据
//
// 返回值:
//   - *FieldDescRegistry: 返回自身以支持链式调用
func (r *FieldDescRegistry) Register(typeName, fieldName string, meta FieldMeta) *FieldDescRegistry {
	if _, ok := r.registry[typeName]; !ok {
		r.registry[typeName] = make(map[string]*FieldMeta)
	}
	r.registry[typeName][fieldName] = &meta
	return r
}

// Get 获取字段描述
//
// 参数:
//   - typeName: 类型名称
//   - fieldName: 字段名称
//
// 返回值:
//   - *FieldMeta: 字段元数据，如果不存在则返回 nil
func (r *FieldDescRegistry) Get(typeName, fieldName string) *FieldMeta {
	if typeMap, ok := r.registry[typeName]; ok {
		return typeMap[fieldName]
	}
	return nil
}

// GetAllForType 获取指定类型的所有字段描述
//
// 参数:
//   - typeName: 类型名称
//
// 返回值:
//   - map[string]*FieldMeta: 字段名到元数据的映射
func (r *FieldDescRegistry) GetAllForType(typeName string) map[string]*FieldMeta {
	if typeMap, ok := r.registry[typeName]; ok {
		return typeMap
	}
	return nil
}

// ApplyRegistry 将注册表中的描述应用到 Builder
//
// 参数:
//   - registry: 字段描述注册表
//   - typeName: 类型名称
//
// 返回值:
//   - *SchemaBuilder: 返回自身以支持链式调用
func (b *SchemaBuilder) ApplyRegistry(registry *FieldDescRegistry, typeName string) *SchemaBuilder {
	if metas := registry.GetAllForType(typeName); metas != nil {
		for fieldName, meta := range metas {
			b.fieldMetas[fieldName] = meta
		}
	}
	return b
}

// =============================================================================
// 便捷函数 - 简化常见用法
// =============================================================================

// QuickSchema 快速生成 Schema（使用默认选项）
//
// 参数:
//   - structInstance: 要转换的结构体实例
//   - fieldMetas: 可选的字段元数据映射
//
// 返回值:
//   - schemaJSON: JSON Schema 字符串
//   - err: 错误信息
//
// 使用示例:
//
//	schema, err := QuickSchema(&User{}, map[string]FieldMeta{
//	    "name": {Title: "用户名", Description: "很长的描述..."},
//	})
func QuickSchema(structInstance interface{}, fieldMetas ...map[string]FieldMeta) (schemaJSON string, err error) {
	builder := NewSchemaBuilder(structInstance).
		WithOptions(GetDefaultOptions())

	if len(fieldMetas) > 0 && fieldMetas[0] != nil {
		for fieldName, meta := range fieldMetas[0] {
			builder.SetFieldMeta(fieldName, meta)
		}
	}

	return builder.Build()
}

// QuickSchemaCompact 快速生成紧凑格式的 Schema
//
// 参数:
//   - structInstance: 要转换的结构体实例
//   - fieldMetas: 可选的字段元数据映射
//
// 返回值:
//   - schemaJSON: JSON Schema 字符串（紧凑格式）
//   - err: 错误信息
func QuickSchemaCompact(structInstance interface{}, fieldMetas ...map[string]FieldMeta) (schemaJSON string, err error) {
	builder := NewSchemaBuilder(structInstance).
		WithOptions(GetCompactOptions())

	if len(fieldMetas) > 0 && fieldMetas[0] != nil {
		for fieldName, meta := range fieldMetas[0] {
			builder.SetFieldMeta(fieldName, meta)
		}
	}

	return builder.Build()
}
