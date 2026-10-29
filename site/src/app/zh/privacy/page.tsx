import type { Metadata } from "next";

import { DocumentLang } from "@/components/document-lang";
import { LegalPage } from "@/components/legal-page";

export const metadata: Metadata = {
  title: "隐私说明",
  description: "Pane 的隐私说明：会出网的请求、本机存储与遥测边界。",
  alternates: { canonical: "https://pane.threetwoa.live/zh/privacy" },
};

export default function PrivacyPage() {
  return (
    <>
      <DocumentLang lang="zh" />
      <LegalPage
        title="隐私说明"
        updated="最后更新：2026-10-26"
        authoritativeLabel="权威版本"
        authoritativeHref="https://github.com/Aafff623/pane/blob/main/docs/privacy.md"
        alternate={{ href: "/privacy", label: "English" }}
      >
        <p>
          Pane 建立在一条规则上：你的数据不关我们的事。没有 Pane 账号，也没有 Pane 后端接收你的额度、花费、密钥或服务商数据。
        </p>

        <h2>会出网的请求</h2>
        <ul>
          <li>服务商自己的 API：只在你启用该服务商时，按其官方工具相同的方式发送该账号的 token/key。</li>
          <li>公开价格表：LiteLLM、models.dev、OpenUsage supplement，匿名 GET，无身份信息。</li>
          <li>更新检查：本项目的 update/latest.json 与 GitHub Releases 兜底，匿名 GET，不携带 App 数据。</li>
          <li>状态页：仅当 Claude 卡片处于 stale/失败且状态探测开启时，读取 status.claude.com 的公开事件摘要。</li>
          <li>本机 Ollama：仅当启用时访问 127.0.0.1。</li>
        </ul>
        <p>没有会话录制、事件流、A/B 标记、自动捕获或遥测。应用不发送任何统计，也不向上游作者的统计项目发送数据。</p>

        <h2>本机存储</h2>
        <p>
          配置、账号、快照、花费台账、皮肤与字体包位于 %APPDATA%\PaneAafff623。密钥优先进入系统凭据库；无法使用时才回退到受保护的本机文件。删除这些文件等于清空 Pane
          的本机记忆。
        </p>

        <h2>遥测</h2>
        <p>本版不含遥测，也没有需要关闭的统计开关。若未来引入任何上报，会先写入这份清单并提供明确开关。</p>

        <h2>联系</h2>
        <p>
          问题与数据请求：<a href="https://github.com/Aafff623/pane/issues">GitHub Issues</a>。
        </p>
      </LegalPage>
    </>
  );
}
