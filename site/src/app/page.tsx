import { HomePage } from "@/components/home-page";

const homeTitle = "Pane — Local-first AI usage dashboard";
const homeDescription =
  "Pane brings quotas, balances, reset times, multiple accounts and local CLI token spend into one floating desktop panel. Credentials stay on your machine.";

export const metadata = {
  title: { absolute: homeTitle },
  description: homeDescription,
  alternates: { canonical: "https://pane.threetwoa.live/", languages: { "zh-CN": "https://pane.threetwoa.live/zh", en: "https://pane.threetwoa.live/", "x-default": "https://pane.threetwoa.live/" } },
  openGraph: {
    type: "website",
    title: homeTitle,
    description: homeDescription,
    url: "https://pane.threetwoa.live/",
  },
};

export default function Home() {
  return <HomePage lang="en" />;
}
