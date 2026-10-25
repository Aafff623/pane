/// Controls for every settings page, lifted out of index.html.
/// The panel adopts these nodes into the skeleton (ids and listeners intact);
/// the popover keeps them in an inert, hidden source. Nothing here is a
/// second UI — `#settings` no longer exists as a surface.
export const SETTINGS_CONTROLS_HTML = `<div class="acc-group open">
        <button class="acc-head"><span data-i18n="settings.general">General</span> <span class="chev">&#8964;</span></button>
        <div class="acc-body"><div class="acc-inner">
          <div class="setting-row">
            <label for="locale" data-i18n="settings.language">Language</label>
            <select id="locale">
              <option value="auto" data-i18n="settings.langAuto">Auto</option>
              <option value="en" data-i18n="settings.langEn">English</option>
              <option value="zh" data-i18n="settings.langZh">中文</option>
              <option value="ru" data-i18n="settings.langRu">Русский</option>
            </select>
          </div>
          <div class="setting-row">
            <label for="interval" data-i18n="settings.refreshEvery">Refresh every</label>
            <input id="interval" type="number" min="1" max="120" value="5" /> <span class="unit" data-i18n="settings.min">min</span>
          </div>
          <div class="setting-row">
            <label class="toggle"><input id="autostart" type="checkbox" /> <span data-i18n="settings.startWithWindows">Start with Windows</span></label>
          </div>
          <div class="setting-row">
            <label for="pinned" data-i18n="settings.trayShows">Tray icon shows</label>
            <select id="pinned"></select>
          </div>
          <div class="setting-row">
            <label for="timeformat" data-i18n="settings.timeFormat">Time format</label>
            <select id="timeformat">
              <option value="auto" data-i18n="settings.timeAuto">Auto</option>
              <option value="12" data-i18n="settings.time12">12-hour</option>
              <option value="24" data-i18n="settings.time24">24-hour</option>
            </select>
          </div>
          <div class="setting-row">
            <label for="appearance" data-i18n="settings.appearance">Appearance</label>
            <select id="appearance">
              <option value="system" data-i18n="settings.appearSystem">System</option>
              <option value="light" data-i18n="settings.appearLight">Light</option>
              <option value="dark" data-i18n="settings.appearDark">Dark</option>
            </select>
          </div>
          <div class="setting-row">
            <label for="ui-font" data-i18n="settings.font">Font</label>
            <input id="ui-font" type="text" spellcheck="false" autocomplete="off" placeholder="Type to filter installed fonts" data-i18n-placeholder="settings.fontPh" />
          </div>
          <div class="setting-row">
            <label for="ui-font-cjk" data-i18n="settings.fontCjk">Chinese font</label>
            <input id="ui-font-cjk" type="text" list="ui-font-cjk-list" spellcheck="false" autocomplete="off" placeholder="System Chinese font" data-i18n-placeholder="settings.fontCjkPh" />
            <datalist id="ui-font-cjk-list"></datalist>
          </div>
          <div class="font-pack-manager"></div>
          <div class="setting-row">
            <span data-i18n="settings.tourReplayLabel">Onboarding tour</span>
            <button class="mini-btn" id="tour-replay" data-i18n="settings.tourReplay">Replay</button>
          </div>
          <div class="setting-row">
            <label class="toggle"><input id="density" type="checkbox" /> <span data-i18n="settings.compact">Compact layout</span></label>
          </div>
          <div class="setting-row">
            <label class="toggle" title="Turn off on slower PCs — replaces the glass refraction with a simple solid look" data-i18n-title="settings.glassTip"><input id="glass" type="checkbox" /> <span data-i18n="settings.glass">Liquid glass effects</span></label>
          </div>
          <div class="setting-row">
            <label class="toggle" title="Skip card entrance motion and the day/night wipe. Windows' own 'Animation effects' setting is still respected." data-i18n-title="settings.reduceAnimTip"><input id="reduce-anim" type="checkbox" /> <span data-i18n="settings.reduceAnim">Reduce animations</span></label>
          </div>
          <div class="setting-row">
            <label class="toggle" title="Play the brand animation the first time Pane is shown after each system boot." data-i18n-title="settings.startupAnimTip"><input id="startup-anim" type="checkbox" /> <span data-i18n="settings.startupAnim">Startup animation</span></label>
            <button class="link-btn" id="splash-replay" type="button" data-i18n="settings.startupAnimReplay">Replay</button>
          </div>
          <div class="setting-row">
            <label for="jump-animation" data-i18n="settings.jumpAnimation">Overview jump</label>
            <select id="jump-animation">
              <option value="smooth" data-i18n="settings.jumpAnimationSmooth">Smooth scroll</option>
              <option value="instant" data-i18n="settings.jumpAnimationInstant">Instant jump</option>
            </select>
          </div>
          <div class="setting-row">
            <label class="toggle"><input id="show-total-spend" type="checkbox" /> <span data-i18n="settings.showSpend">Show Total Spend card</span></label>
          </div>
          <div class="setting-row">
            <label for="spend-tier-mode" data-i18n="settings.spendTiers">Spend icon levels</label>
            <select id="spend-tier-mode">
              <option value="builtin" data-i18n="settings.spendTiersBuiltin">Built-in (100M / 250M / 500M per day)</option>
              <option value="custom" data-i18n="settings.spendTiersCustom">Custom</option>
            </select>
          </div>
          <div class="setting-row" id="spend-tier-custom" hidden>
            <label data-i18n="settings.spendTiersLevels">Medium / High / Max (M tokens per day)</label>
            <input id="spend-tier-medium" type="number" min="1" max="100000" step="10" />
            <input id="spend-tier-high" type="number" min="1" max="100000" step="10" />
            <input id="spend-tier-max" type="number" min="1" max="100000" step="10" />
          </div>
          <div class="setting-row">
            <label class="toggle" title="Unfinished features under active development. Skins are available independently of this switch." data-i18n-title="settings.experimentalTip"><input id="experimental-features" type="checkbox" /> <span data-i18n="settings.experimental">Experimental features</span></label>
          </div>
          <div class="setting-row">
            <label class="toggle" title="Category chips wrap into a two-row grid (3 per row, 6 max) instead of one compact scrollable row; extra categories hide behind …" data-i18n-title="settings.overviewCatFullTip"><input id="overview-cat-full" type="checkbox" /> <span data-i18n="settings.overviewCatFull">Show all overview categories</span></label>
          </div>
          <div class="setting-row">
            <label class="toggle" title="Automatically claim Qoder CN's daily 100-credit benefit once per claim window. Off by default: it performs a write on your account." data-i18n-title="settings.qoderCheckinTip"><input id="qoder-checkin" type="checkbox" /> <span data-i18n="settings.qoderCheckin">Qoder CN daily check-in</span></label>
          </div>
          <div class="setting-row">
            <label data-i18n-title="settings.agentSkillTip" title="Copies a small skill folder into ~/.agents/skills/pane so your coding agent (Claude Code, Codex, Cursor, ZCode…) can diagnose Pane problems itself through the local read-only API. Shows a plan first; existing files are backed up once." data-i18n="settings.agentSkill">AI agent troubleshooting skill</label>
            <button class="mini-btn" id="install-agent-skill" data-i18n="settings.agentSkillInstall">Install</button>
          </div>
        </div></div>
      </div>

      <div class="acc-group">
        <button class="acc-head"><span data-i18n="settings.keyvault">API key vault</span> <span class="chev">&#8964;</span></button>
        <div class="acc-body"><div class="acc-inner">
          <div id="kv-vault-bar"></div>
          <div id="kv-recovery-panel"></div>
          <div id="keyvault-rows"></div>
          <div class="setting-row kv-add-row">
            <input id="kv-service" type="text" data-i18n-placeholder="settings.kvService" placeholder="Service (e.g. tavily)" spellcheck="false" />
            <input id="kv-note" type="text" data-i18n-placeholder="settings.kvNotePlaceholder" placeholder="Note (optional)" spellcheck="false" />
            <input id="kv-key" type="password" data-i18n-placeholder="settings.kvKey" placeholder="API key" spellcheck="false" autocomplete="off" />
            <button class="mini-btn" id="kv-add-btn" data-i18n="settings.kvAdd">Add</button>
          </div>
          <p class="kv-hint" data-i18n="settings.kvHint">Keys are sealed with your master password (Argon2id + AES-256-GCM); service, label and note stay readable so the locked vault still shows what it holds. Cards also monitor keys from environment variables and the ZCode CLI config, and mark vault keys 🔒 while the vault is locked.</p>
        </div></div>
      </div>

      <div class="acc-group">
        <button class="acc-head"><span data-i18n="settings.shortcutManage">快捷键管理</span> <span class="chev">&#8964;</span></button>
        <div class="acc-body"><div class="acc-inner">
          <p class="settings-note" data-i18n="settings.shortcutManageHint">唤醒快捷键可以全局使用；其他快捷键只在 Pane 面板已唤醒并获得焦点时生效。</p>
          <p class="settings-note" data-i18n="settings.shortcutCaptureHint">点击输入框后直接按下组合键即可设置。</p>
          <div id="local-shortcuts"></div>
        </div></div>
      </div>

      <div class="acc-group">
        <button class="acc-head"><span data-i18n="settings.notifications">Notifications</span> <span class="chev">&#8964;</span></button>
        <div class="acc-body"><div class="acc-inner">
          <p class="settings-note" data-i18n="settings.notifyNote">
            Windows toasts when a quota worsens — once per metric per reset period.
          </p>
          <div class="setting-row">
            <label class="toggle"><input id="notify-almost" type="checkbox" /> <span data-i18n="settings.notifyAlmost">Almost out (&lt;10% left)</span></label>
          </div>
          <div class="setting-row">
            <label class="toggle"><input id="notify-close" type="checkbox" /> <span data-i18n="settings.notifyClose">Cutting it close</span></label>
          </div>
          <div class="setting-row">
            <label class="toggle"><input id="notify-runout" type="checkbox" /> <span data-i18n="settings.notifyRunout">Will run out</span></label>
          </div>
          <div class="setting-row">
            <label class="toggle"><input id="notify-resetsoon" type="checkbox" /> <span data-i18n="settings.notifyResetSoon">5-hour window resetting soon</span></label>
          </div>
        </div></div>
      </div>

      <div class="acc-group">
        <button class="acc-head"><span data-i18n="settings.privacy">Privacy</span> <span class="chev">&#8964;</span></button>
        <div class="acc-body"><div class="acc-inner">
          <p class="settings-note" data-i18n="settings.privacyNote">
            One anonymous "alive today" ping and per-provider success/failure
            counts, once a day, under a random ID attached to nothing. No
            usage amounts, no spend, no IPs stored. Full details in
            docs/privacy.md.
          </p>
          <div class="setting-row">
            <label class="toggle"><input id="telemetry" type="checkbox" /> <span data-i18n="settings.telemetry">Share anonymous usage statistics</span></label>
          </div>
          <div class="setting-row">
            <label class="toggle" title="During Presentation Settings, exclusive fullscreen, or remote control, tray percentages hide. The Pane icon and starred provider logos stay. A Teams/Zoom window share is not detected. Off by default." data-i18n-title="settings.hideSharingTip"><input id="hide-while-sharing" type="checkbox" /> <span data-i18n="settings.hideSharing">Hide tray numbers while screen sharing</span></label>
          </div>
          <div class="setting-row">
            <label class="toggle" title="Show the Usage Trend bar on provider cards that log token usage. Off by default — enable this to see quota history on cards that record it." data-i18n-title="settings.showTrendTip"><input id="show-trend" type="checkbox" /> <span data-i18n="settings.showTrend">Show usage trend on cards</span></label>
          </div>
        </div></div>
      </div>

      <div class="acc-group">
        <button class="acc-head"><span data-i18n="settings.network">Network</span> <span class="chev">&#8964;</span></button>
        <div class="acc-body"><div class="acc-inner">
          <div class="setting-row">
            <label class="toggle"><input id="proxy-enabled" type="checkbox" /> <span data-i18n="settings.useProxy">Use proxy</span></label>
          </div>
          <div class="setting-row">
            <label for="proxy-url" data-i18n="settings.proxyUrl">Proxy URL</label>
            <input id="proxy-url" type="text" placeholder="socks5://host:1080" spellcheck="false" />
          </div>
          <p class="settings-note" data-i18n="settings.networkNote">
            Applies after the app restarts. Local usage API always runs at
            http://127.0.0.1:6736/v1/usage
          </p>
        </div></div>
      </div>

      <div class="acc-group">
        <button class="acc-head"><span data-i18n="settings.advanced">Advanced</span> <span class="chev">&#8964;</span></button>
        <div class="acc-body"><div class="acc-inner">
          <p class="settings-note" data-i18n="settings.advancedNote">
            Restores every preference to its default and re-detects installed
            tools. API keys and your usage history stay. Proxy changes still
            need a restart.
          </p>
          <button id="reset-all-settings" type="button" class="danger-btn" data-i18n="settings.resetAll">
            Reset all settings
          </button>
        </div></div>
      </div>`;

export function mountSettingsControls(): HTMLElement {
  const holder = document.createElement("div");
  holder.id = "settings-controls-source";
  holder.hidden = true;
  holder.innerHTML = SETTINGS_CONTROLS_HTML;
  document.body.append(holder);
  return holder;
}
