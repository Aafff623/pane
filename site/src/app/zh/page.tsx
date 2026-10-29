import { DocumentLang } from "@/components/document-lang";
import { HomePage } from "@/components/home-page";
import { JsonLd, faqSchema, softwareApplicationSchema } from "@/components/site/json-ld";
import { homepageFaqQA } from "@/components/blocks/faq";

const homeTitle = "Pane — 本地优先的 AI 用量工作台";
const homeDescription =
  "Pane 把 AI 工具的额度、余额、重置时间、多账号与本地 CLI Token 花费汇集到一个随时可唤出的桌面面板。凭据保存在本机。";

export const metadata = {
  title: { absolute: homeTitle },
  description: homeDescription,
  alternates: { canonical: "https://pane.threetwoa.live/zh" },
};

export default function Home() {
  return (
    <>
      <JsonLd data={softwareApplicationSchema} />
      <JsonLd data={faqSchema(homepageFaqQA)} />
      <DocumentLang lang="zh" />
      <HomePage lang="zh" />
    </>
  );
}
