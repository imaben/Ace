# Sequel Ace → Ace 功能对照清单

> 对照仓库：https://github.com/Sequel-Ace/Sequel-Ace
> 状态：`[ ]` 未做 · `[~]` 进行中 · `[x]` 已对齐  
> 规则：**未勾选项不得宣称完成；每项必须对照源码/XIB 验收。**

## A. 连接与收藏夹（ConnectionView）

- [x] TCP/IP 连接（host/user/password/port/database）
- [x] Socket 连接（socket path）
- [x] SSH Tunnel（ssh host/user/password/key/port/remote socket）
- [x] AWS IAM（profile/region + 自动 SSL）— 表单齐，驱动侧 token 已接
- [x] Vault OIDC（vault host/port/mount/role）— 表单齐，OIDC 浏览器回调仍 stub
- [x] SSL：Require SSL、CA/客户端证书/密钥文件
- [x] RSA public key 请求（caching_sha2 非 SSL）
- [x] 时区：Server / System / Fixed
- [x] 连接名称 + 颜色标签（colorIndex）
- [~] 收藏夹树：分组 / 展开折叠 / 拖拽排序 — 已有分组与列表，缺拖拽与折叠
- [~] 收藏夹搜索、去重、复制、删除确认 — 搜索/复制/删除确认已有，缺去重提示
- [x] 收藏夹导入/导出（Sequel Ace / Sequel Pro plist）
- [ ] Keychain 存密码
- [x] 连接字符串解析
- [ ] 断线检测 / Keep-Alive / 自动重连 / 丢失对话框
- [ ] SSH Askpass / 主机指纹确认 / ssh_config
- [x] 基础连接表单 + Demo SQLite

## B. 主窗口 / 会话（SPDatabaseDocument / TabManager）

- [ ] 多窗口「New Connection Window」
- [ ] 多标签「New Connection Tab / Duplicate」
- [x] 标题栏按收藏夹颜色着色
- [x] 数据库选择器（Go to Database）— 侧栏库列表切换
- [~] 表列表：表/视图/过程/函数、过滤、置顶、类型图标 — 过滤与类型图标已有，缺置顶
- [ ] 表历史前进/后退
- [ ] `.spf` / `.spfs` 会话保存打开
- [~] 状态栏（行数、耗时、连接信息）
- [x] 工具栏切换六视图
- [x] 单窗口单会话基础侧栏 + 视图标签

## C. Structure（SPTableStructure / Indexes）

- [x] 字段表格：Field/Type/Length/Unsigned/Zerofill/Binary/Null/Key/Default/Extra/Encoding/Collation/Comment — 只读网格
- [ ] 字段增删改、重排
- [ ] 自增规则
- [~] 索引管理面板（IndexesView）— 只读列表
- [ ] 表引擎/字符集/自增值选项
- [ ] 校验与 DDL 预览

## D. Content（SPTableContent）

- [~] 分页浏览 + 每页行数偏好 — Prev/Next 分页已有，偏好设置未做
- [ ] 虚拟滚动大表
- [x] 列排序 — 点击表头 ASC/DESC
- [~] 规则过滤器（AND/OR 嵌套）+ 过滤器收藏 — 已有原始 WHERE 过滤条，缺可视化规则编辑器
- [ ] 单元格右键快速过滤
- [~] 内联编辑、新增/复制/删除行 — 点选单元格编辑 / 增删行已接（需主键）；复制行未做
- [ ] 字段编辑器：文本/JSON/BLOB hex/图片/几何/PHP 序列化
- [ ] 外键跳转
- [ ] 复制 CSV/SQL/TSV、Copy with Column Names、INSERT（含/不含 auto_inc）
- [ ] Insert NULL、Edit All Fields in Pop-up Sheet
- [ ] Display Binary Data as Hex
- [x] 只读网格展示首屏行

## E. Relations / Triggers / Table Info

- [~] 外键列表、新增、删除、ON UPDATE/DELETE — 只读列表
- [~] 触发器增删改（时机/事件/语句）— 只读列表
- [~] Table Info：行数/大小/引擎/CREATE SQL/注释编辑 — 只读展示

## F. Custom Query（SPCustomQuery / SPTextView）

- [ ] SQL 编辑器：语法高亮、括号匹配、缩进、主题（.spTheme）
- [ ] 自动补全（库/表/字段/关键字）
- [ ] 执行当前/选中/全部语句、取消（KILL QUERY）
- [~] 多结果集、结果可编辑（单表）— 多语句已执行并展示末个结果集
- [x] EXPLAIN
- [~] 查询历史、查询收藏 — 历史侧栏已接；收藏未做
- [ ] 错误定位、执行进度
- [~] 简易输入框 + 单结果网格

## G. 数据库 / 表操作

- [ ] 新建/重命名/复制/删除数据库
- [ ] 表：复制/重命名/清空/删除/打开新标签或窗口
- [ ] Analyze / Check / Optimize / Repair / Flush / Checksum
- [ ] Copy Create Table Syntax / Show Create Table Syntax
- [ ] 新建视图/存储过程/函数并编辑
- [ ] Refresh Databases / Tables
- [ ] Open Database in New Tab

## H. 服务器工具

- [ ] Process List（自动刷新、KILL）
- [ ] Server Variables
- [ ] User Accounts（用户/主机/权限/资源限制）
- [ ] Console（全部 SQL 日志、过滤、清空、保存）
- [ ] Navigator（全库表字段树）
- [ ] Encoding 切换、Flush Privileges、Shutdown Server

## I. 导入 / 导出

- [ ] Export：SQL/CSV/XML/HTML/PDF/Dot
- [ ] 范围：整库/选中表/查询结果/过滤内容
- [ ] 文件名 token、编码、gzip/bzip2
- [ ] Import SQL 流式、CSV 字段映射（INSERT/REPLACE/UPDATE）
- [ ] Import from Clipboard
- [ ] 进度与取消

## J. 扩展与集成

- [ ] Bundle 系统（17 个默认包 + 编辑器）
- [ ] MCP Server（19 tools，只读开关）
- [ ] MySQL Help / Online Help / FAQ
- [ ] Print（HTML 模板）
- [ ] 检查更新
- [ ] 本地化（16 语言）

## K. 偏好设置

- [ ] General / Tables / Editor / Files / Network / Notifications / MCP
- [ ] 快捷键自定义、明暗外观

## L. 多数据库（计划扩展，Sequel Ace 本身仅 MySQL/MariaDB）

- [ ] PostgreSQL 驱动与方言能力开关
- [ ] SQLite 驱动与方言能力开关

---

### 当前冲刺顺序

1. ~~A 连接窗口完整字段与收藏夹树~~（表单五类型齐；收藏夹拖拽/确认/plist 待补）  
2. B 主窗口：库选择器、多标签、置顶、状态栏细化  
3. C Structure：可编辑字段/索引 + DDL 预览  
4. D Content：排序/过滤/内联编辑  
5. F Query 编辑器升级  
6. 其余按清单推进，每完成一项勾选并对照验收
