//! 加密级随机数：出题（新数字的位置与数值）全部走这里。

use anyhow::{Context, Result};
use rand::rngs::{StdRng, SysRng};
use rand::seq::IndexedRandom;
use rand::{RngExt, SeedableRng};

/// 游戏使用的随机数发生器。
///
/// [`StdRng`] 是 `rand` 提供的密码学安全伪随机数发生器（当前实现为 ChaCha12 流密码），
/// [`SysRng`] 是操作系统的熵源；这里用 256 位新鲜的操作系统熵做种子，既不依赖时间戳，
/// 也不会退化成可预测的种子。
pub struct GameRng(StdRng);

impl GameRng {
    /// 用操作系统熵源做种子初始化发生器。
    pub fn from_os_entropy() -> Result<Self> {
        let rng =
            StdRng::try_from_rng(&mut SysRng).context("无法从操作系统熵源初始化随机数发生器")?;
        Ok(Self(rng))
    }

    /// 固定种子版本，便于测试复现同一局游戏。
    #[cfg(test)]
    pub fn from_seed(seed: [u8; 32]) -> Self {
        Self(StdRng::from_seed(seed))
    }

    /// 从 `items` 中均匀随机地取出一个元素。
    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> Option<&'a T> {
        items.choose(&mut self.0)
    }

    /// 以 `numerator / denominator` 的概率返回 `true`。
    pub fn chance(&mut self, numerator: u32, denominator: u32) -> bool {
        self.0.random_ratio(numerator, denominator)
    }
}
