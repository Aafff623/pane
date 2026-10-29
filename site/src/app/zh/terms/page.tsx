import type { Metadata } from "next";

import { DocumentLang } from "@/components/document-lang";
import { LegalPage } from "@/components/legal-page";

export const metadata: Metadata = {
  title: "条款与许可",
  description: "Pane 的条款、MIT 许可与数据说明。",
  alternates: { canonical: "https://pane.threetwoa.live/zh/terms/" },
};

export default function TermsPage() {
  return (
    <>
      <DocumentLang lang="zh" />
      <LegalPage
        title="条款与许可"
        updated="最后更新：2026-10-26"
        authoritativeLabel="权威许可文本"
        authoritativeHref="https://github.com/Aafff623/pane/blob/main/LICENSE"
        alternate={{ href: "/terms", label: "English" }}
      >
        <h2>许可</h2>
        <p>
          Pane 以 MIT 许可发布。你可以使用、复制、修改、合并、发布、分发、再许可和/或销售软件的副本，但须在副本或实质性部分中保留版权声明与许可声明。
        </p>

        <h2>按现状提供</h2>
        <p>
          软件按“现状”提供，不附带任何明示或暗示的担保，包括但不限于适销性、特定用途适用性和非侵权担保。作者或版权持有人不对任何索赔、损害或其他责任负责。
        </p>

        <h2>第三方服务</h2>
        <p>
          Pane 读取你本机的凭据并访问你启用的服务商 API。各服务商的使用受其自己的条款与隐私政策约束；Pane 不隶属于任何服务商，也不代表它们作出承诺。
        </p>

        <h2>数据与备份</h2>
        <p>本机数据由你控制。删除 %APPDATA%\PaneAafff623 或卸载会移除 Pane 的本机配置与台账；请自行备份需要保留的内容。</p>

        <h2>联系</h2>
        <p>
          问题与许可相关询问：<a href="https://github.com/Aafff623/pane/issues">GitHub Issues</a>。
        </p>
      </LegalPage>
    </>
  );
}
