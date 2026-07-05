interface Props {
  onInstall: () => void;
  installing: boolean;
}

export function InstallBanner({ onInstall, installing }: Props) {
  return (
    <div className="auth-banner install-banner">
      <div className="auth-text">
        <strong>未检测到 arkcli</strong>
        <div className="auth-detail">
          安装命令：<code>npm i -g @volcengine/ark-cli</code>
          <br />
          安装后运行 <code>arkcli auth login volc-sso</code> 登录，再点刷新。
        </div>
      </div>
      <button className="primary-btn" onClick={onInstall} disabled={installing}>
        {installing ? "正在打开…" : "在终端安装"}
      </button>
    </div>
  );
}
