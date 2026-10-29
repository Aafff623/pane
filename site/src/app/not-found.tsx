import Link from "next/link";

import { Button } from "@/components/ui/button";

export default function NotFound() {
  return (
    <section className="container flex min-h-[70vh] max-w-3xl flex-col items-start justify-center py-24">
      <p className="text-muted-foreground text-base font-semibold tracking-wide uppercase">404 · Page not found</p>
      <h1 className="mt-4 text-3xl tracking-tight sm:text-4xl md:text-5xl">这个页面不在 Pane 官网里。</h1>
      <p className="text-muted-foreground mt-4 max-w-2xl text-base leading-relaxed">
        链接可能已经过期。回到首页查看功能和下载入口，或者到 GitHub 提交反馈。
      </p>
      <div className="mt-8 flex flex-wrap gap-4">
        <Button asChild>
          <Link href="/">返回首页</Link>
        </Button>
        <Button asChild variant="outline">
          <a href="https://github.com/Aafff623/pane/issues" target="_blank" rel="noopener noreferrer">
            提交反馈
          </a>
        </Button>
      </div>
    </section>
  );
}
