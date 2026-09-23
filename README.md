# Moe2048

使用 [Rust](https://rust-lang.org/zh-CN/) + [Tauri 2](https://tauri.app/) + [Vue 3](https://cn.vuejs.org/) + [TypeScript](https://www.typescriptlang.org/) 实现的 2048 小游戏。

游戏逻辑全部在 Rust 后端实现，前端仅负责渲染和与用户交互，实现了 MVVM 架构。

## 控制方式

| 操作   | 键盘快捷键 |
| --- | --- |
| 移动   | `↑` `↓` `←` `→` 或 `W` `A` `S` `D` 或在触摸屏上滑动 |
| 撤销   | `Ctrl` + `Z` |
| 重新开始 | `R` |

## 构建

```sh
pnpm install
pnpm tauri dev        # debug模式运行
pnpm tauri build      # release构建并打包
```

## 项目结构

```
src-tauri/src/
  main.rs              入口点
  lib.rs               核心库
  commands.rs          前后端交互接口
  game/
    geometry.rs        坐标运算
    tile.rs            Tile类的实现
    direction.rs       方向及相关运算
    rng.rs             大肥鱼写的随机数引擎（可以用其他轮子替代）
    board.rs           棋盘及游戏规则的实现
    state.rs           维护状态

src/
  api/types.ts          数据类型定义
  api/game.ts           前后端交互接口
  composables/          前端逻辑
    useGame.ts          
    moveAnimation.ts    
    useKeyboard.ts      
    useSwipe.ts         
    useBestScore.ts     
  components/           前端基本框架
  styles/main.css       前端美化

scripts/
  check-move-animation.ts   前端测试
```
