# 小说与听书精简版

基于 ch6vip/fqapp 的 Android 版本。首页保留推荐、小说、听书；阅读页右上角的“更多阅读操作”可切换段评气泡，并与原阅读设置共用同一项偏好。旧短剧和漫画数据不会因界面筛选而被删除。本版本保留共用播放器及 Rust 核心，主要精简操作入口，不承诺安装包体积明显变小。

## 持续构建与固定签名

工作流 `Reader Lite Android APK` 在 `master` 分支更新或手动触发时运行。首次运行前，仓库所有者须在 Settings → Secrets and variables → Actions 中配置：

* Secrets：`RELEASE_KEYSTORE_BASE64`（keystore 文件的 base64）、`RELEASE_STORE_PASSWORD`、`RELEASE_KEY_ALIAS`、`RELEASE_KEY_PASSWORD`。
* Variables：`READER_LITE_SIGNER_SHA256`（该密钥证书 SHA-256 指纹）。

密钥和密码应由所有者长期安全备份，绝不可提交源码或公开分享。工作流缺少签名信息或证书指纹不一致时会失败，不输出供安装的未签名包。构建完成后，从 Actions 对应运行记录的 artifact 下载两个架构的 APK 和签名校验结果。使用同一 applicationId、同一签名且版本号不降低的精简版之间才能覆盖安装；原作者版本通常使用不同签名，不能覆盖安装，切换前请先备份数据。

## 同步上游

工作流 `Check upstream updates` 每天检查上游 `ch6vip/fqapp` 的 `master`，创建独立分支和待审核的合并请求；它不会直接把上游修改合入精简版。如发生合并冲突，工作流失败并保留精简版原样，需人工处理。仓库必须在 Actions 设置中允许 GitHub Actions 创建 pull request；每次合并前应检查原有功能与签名配置，合并后由构建工作流生成新版 APK。上游更新可能触及相同文件，无法保证完全自动合并。
