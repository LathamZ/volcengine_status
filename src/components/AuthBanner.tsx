interface Props {
  message?: string;
  onLogin: () => void;
  logging: boolean;
}

export function AuthBanner({ message, onLogin, logging }: Props) {
  return (
    <div className="auth-banner">
      <div className="auth-text">
        <strong>SSO 会话已失效</strong>
        <div className="auth-detail">
          {message || "运行 arkcli auth login 重新登录火山引擎账号后，点击刷新。"}
        </div>
      </div>
      <button className="primary-btn" onClick={onLogin} disabled={logging}>
        {logging ? "正在打开…" : "登录"}
      </button>
    </div>
  );
}
