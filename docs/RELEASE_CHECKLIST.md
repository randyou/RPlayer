# RPlayer 发布检查清单

每次 release 按顺序执行，全部通过后再提交。

## 1. 版本

- [ ] 在 `Cargo.toml` 修改 `version`。
- [ ] 确认源码中没有硬编码版本：
  - 搜索 `v0.`、`FileVersion`、`ProductVersion`。
  - UI 必须使用 `env!("CARGO_PKG_VERSION")`。
  - `build.rs` 必须从 `CARGO_PKG_VERSION` 生成 exe 版本。
- [ ] 关闭正在运行的 `rplayer.exe`，否则链接会报 `os error 5`。

## 2. 测试与构建

```powershell
Get-Process rplayer -ErrorAction SilentlyContinue | Stop-Process -Force
$env:PATH='D:\Workspace\_tools\rust\bin;C:\Users\Randy\AppData\Local\Microsoft\WinGet\Packages\BrechtSanders.WinLibs.POSIX.UCRT_Microsoft.Winget.Source_8wekyb3d8bbwe\mingw64\bin;' + $env:PATH
$env:CARGO_BUILD_JOBS='1'
cargo test --release --manifest-path D:\Workspace\RPlayer\Cargo.toml
cargo build --release --manifest-path D:\Workspace\RPlayer\Cargo.toml --bin rplayer
```

- [ ] 测试全部通过。
- [ ] release 构建通过。
- [ ] 不运行 `cargo fmt --all`；仓库当前不是 rustfmt-clean。

## 3. UI 与元数据

- [ ] 启动 `target\release\rplayer.exe`。
- [ ] 检查侧栏、设置、关于的版本一致。
- [ ] 检查本次改动涉及的页面截图，确认无错位、闪烁、遮挡。
- [ ] 检查 exe 属性：

```powershell
(Get-Item 'D:\Workspace\RPlayer\target\release\rplayer.exe').VersionInfo |
  Select-Object FileVersion, ProductVersion
```

- [ ] 文件版本和产品版本与 Cargo 版本一致（形如 `0.13.0.0`）。

## 4. 文档

- [ ] 更新 `README.md` 版本记录与特性列表。
- [ ] 新增或更新 `docs/VX.Y_PLAN.md`。
- [ ] 更新 `dist/RPlayer/使用说明.txt`。
- [ ] 如架构、模块或构建方式变化，更新 `AGENTS.md`。
- [ ] 不把测试截图留在 `docs/`。

## 5. 打包

```powershell
Copy-Item 'D:\Workspace\RPlayer\target\release\rplayer.exe' 'D:\Workspace\RPlayer\dist\RPlayer\RPlayer.exe' -Force
Copy-Item 'D:\Workspace\RPlayer\README.md' 'D:\Workspace\RPlayer\dist\RPlayer\README.md' -Force
Remove-Item 'D:\Workspace\RPlayer\dist\RPlayer\docs' -Recurse -Force -ErrorAction SilentlyContinue
Copy-Item 'D:\Workspace\RPlayer\docs' 'D:\Workspace\RPlayer\dist\RPlayer\docs' -Recurse -Force
Remove-Item 'D:\Workspace\RPlayer\dist\RPlayer\brand' -Recurse -Force -ErrorAction SilentlyContinue
Copy-Item 'D:\Workspace\RPlayer\assets\brand' 'D:\Workspace\RPlayer\dist\RPlayer\brand' -Recurse -Force
Remove-Item 'D:\Workspace\RPlayer\dist\RPlayer-windows-x64.zip' -Force -ErrorAction SilentlyContinue
```

用 Python 生成 zip、校验完整性和 SHA-256：

```python
from pathlib import Path
from zipfile import ZipFile, ZIP_DEFLATED
import hashlib

root = Path(r'D:\Workspace\RPlayer\dist\RPlayer')
out = Path(r'D:\Workspace\RPlayer\dist\RPlayer-windows-x64.zip')
with ZipFile(out, 'w', ZIP_DEFLATED, compresslevel=9) as z:
    for p in sorted(root.rglob('*')):
        if p.is_file():
            z.write(p, Path('RPlayer') / p.relative_to(root))

print('size', out.stat().st_size)
print('sha256', hashlib.sha256(out.read_bytes()).hexdigest())
print('integrity', ZipFile(out).testzip())
```

- [ ] `integrity` 为 `None`。
- [ ] 记录 zip 大小与 SHA-256。

## 6. 提交

```powershell
cd D:\Workspace\RPlayer
git status --short
git diff --check
git add -A
git commit -m "release: vX.Y.Z <主题>"
```

- [ ] 提交信息包含版本号和主题。
- [ ] 提交后工作区干净。
- [ ] 最终回复包含版本、主要改动、测试数量、包大小、SHA-256、提交号。
