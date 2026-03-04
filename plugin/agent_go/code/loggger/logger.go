package loggger

import (
	"os"
	"sync"

	"github.com/natefinch/lumberjack"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

var (
	logger *zap.Logger
	sugar  *zap.SugaredLogger
	once   sync.Once
)

// Config 日志配置
type Config struct {
	// Level 日志级别: debug, info, warn, error
	Level string
	// FilePath 日志文件路径，为空则只输出到控制台
	FilePath string
	// MaxSize 单个日志文件最大大小(MB)
	MaxSize int
	// MaxBackups 保留的旧日志文件最大数量
	MaxBackups int
	// MaxAge 保留旧日志文件的最大天数
	MaxAge int
	// Compress 是否压缩旧日志文件
	Compress bool
}

// DefaultConfig 返回默认日志配置
func DefaultConfig() *Config {
	return &Config{
		Level:      "info",
		MaxSize:    100,
		MaxBackups: 5,
		MaxAge:     30,
		Compress:   true,
	}
}

// Init 初始化全局 Logger
func Init(cfg *Config) {
	once.Do(func() {
		if cfg == nil {
			cfg = DefaultConfig()
		}

		// 解析日志级别
		level := parseLevel(cfg.Level)

		// 编码器配置
		encoderConfig := zapcore.EncoderConfig{
			TimeKey:        "time",
			LevelKey:       "level",
			NameKey:        "logger",
			CallerKey:      "caller",
			FunctionKey:    zapcore.OmitKey,
			MessageKey:     "msg",
			StacktraceKey:  "stacktrace",
			LineEnding:     zapcore.DefaultLineEnding,
			EncodeLevel:    zapcore.CapitalColorLevelEncoder,
			EncodeTime:     zapcore.TimeEncoderOfLayout("2006-01-02 15:04:05.000"),
			EncodeDuration: zapcore.StringDurationEncoder,
			EncodeCaller:   zapcore.ShortCallerEncoder,
		}

		// 控制台输出
		consoleEncoder := zapcore.NewConsoleEncoder(encoderConfig)
		consoleSyncer := zapcore.AddSync(os.Stdout)

		var cores []zapcore.Core
		cores = append(cores, zapcore.NewCore(consoleEncoder, consoleSyncer, level))

		// 文件输出（如果配置了文件路径）
		if cfg.FilePath != "" {
			// 文件使用 JSON 编码，不带颜色
			fileEncoderConfig := encoderConfig
			fileEncoderConfig.EncodeLevel = zapcore.CapitalLevelEncoder

			fileEncoder := zapcore.NewJSONEncoder(fileEncoderConfig)
			fileSyncer := zapcore.AddSync(&lumberjack.Logger{
				Filename:   cfg.FilePath,
				MaxSize:    cfg.MaxSize,
				MaxBackups: cfg.MaxBackups,
				MaxAge:     cfg.MaxAge,
				Compress:   cfg.Compress,
			})
			cores = append(cores, zapcore.NewCore(fileEncoder, fileSyncer, level))
		}

		core := zapcore.NewTee(cores...)
		logger = zap.New(core, zap.AddCaller(), zap.AddCallerSkip(1))
		sugar = logger.Sugar()
	})
}

// parseLevel 解析日志级别字符串
func parseLevel(level string) zapcore.Level {
	switch level {
	case "debug":
		return zapcore.DebugLevel
	case "info":
		return zapcore.InfoLevel
	case "warn":
		return zapcore.WarnLevel
	case "error":
		return zapcore.ErrorLevel
	default:
		return zapcore.InfoLevel
	}
}

// getLogger 获取全局 Logger，如未初始化则使用默认配置
func getLogger() *zap.Logger {
	if logger == nil {
		Init(nil)
	}
	return logger
}

// getSugar 获取全局 SugaredLogger
func getSugar() *zap.SugaredLogger {
	if sugar == nil {
		Init(nil)
	}
	return sugar
}

// GetLogger 获取原始 zap.Logger（供需要高性能场景使用）
func GetLogger() *zap.Logger {
	return getLogger()
}

// Info 输出 Info 级别日志
func Info(msg string, fields ...zap.Field) {
	getLogger().Info(msg, fields...)
}

// Warn 输出 Warn 级别日志
func Warn(msg string, fields ...zap.Field) {
	getLogger().Warn(msg, fields...)
}

// Error 输出 Error 级别日志
func Error(msg string, fields ...zap.Field) {
	getLogger().Error(msg, fields...)
}

// Debug 输出 Debug 级别日志
func Debug(msg string, fields ...zap.Field) {
	getLogger().Debug(msg, fields...)
}

// Infof 格式化输出 Info 级别日志
func Infof(template string, args ...interface{}) {
	getSugar().Infof(template, args...)
}

// Warnf 格式化输出 Warn 级别日志
func Warnf(template string, args ...interface{}) {
	getSugar().Warnf(template, args...)
}

// Errorf 格式化输出 Error 级别日志
func Errorf(template string, args ...interface{}) {
	getSugar().Errorf(template, args...)
}

// Debugf 格式化输出 Debug 级别日志
func Debugf(template string, args ...interface{}) {
	getSugar().Debugf(template, args...)
}

// Sync 刷新日志缓冲区，应在程序退出前调用
func Sync() {
	if logger != nil {
		_ = logger.Sync()
	}
}
