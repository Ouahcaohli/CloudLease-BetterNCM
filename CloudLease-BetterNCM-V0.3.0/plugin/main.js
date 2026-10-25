(() => {
  const ID = 'cloudlease-native-launcher';
  if (document.getElementById(ID)) return;

  const button = document.createElement('button');
  button.id = ID;
  button.type = 'button';
  button.textContent = '租期工作台';
  button.title = '打开或收纳网易云旁置租期工作台';
  Object.assign(button.style, {
    position: 'fixed', right: '18px', top: '74px', zIndex: '2147483647',
    padding: '8px 12px', border: '1px solid rgba(255,255,255,.18)',
    borderRadius: '8px', background: '#252525', color: '#fff',
    fontSize: '13px', cursor: 'pointer', boxShadow: '0 4px 14px rgba(0,0,0,.22)'
  });

  button.addEventListener('click', () => {
    try {
      const api = window.betterncm_native && window.betterncm_native.cloudlease_native;
      if (api && typeof api.toggle_panel === 'function') {
        api.toggle_panel();
      } else {
        console.error('[CloudLease] Native API 未注册。请确认 native.dll 已构建并安装，且重启过网易云音乐。');
        alert('租期工作台的原生模块还没有加载。请确认已安装 native.dll，并完全退出后重新启动网易云音乐。');
      }
    } catch (error) {
      console.error('[CloudLease] 无法打开原生工作台：', error);
      alert('无法打开租期工作台，请查看 BetterNCM 控制台日志。');
    }
  });

  document.body.appendChild(button);
  console.info('[CloudLease] V0.3.0 launcher loaded.');
})();
