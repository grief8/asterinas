# 星绽（Asterinas）项目简介

<p align="center">
    <img src="images/logo_cn.svg" alt="asterinas-logo" width="620"><br>
</p>

欢迎来到星绽（Asterinas）项目！
星绽是一个使用 Rust 编写、与 Linux 兼容的安全、快速、通用的操作系统（OS）内核，
同时配套两个核心子项目：为安全内核开发打造的低层框架 OSTD，以及面向 OS 开发者的命令行工具 OSDK。

- Asterinas Kernel：提供与 Linux 兼容的 ABI，可无缝运行 Linux 程序，同时在内存安全与可靠性上更进一步。
- Asterinas OSTD：一个用于安全 Rust OS 开发的基础框架（类似于 Rust `std` 之于用户态），为 CPU、内存管理、中断、并发等底层能力提供健壮且可验证的抽象。
- Asterinas OSDK：一个扩展 Cargo 的命令行工具，简化 OS 项目的创建、构建、测试与运行流程。

## 核心组件

### 1. Asterinas Kernel（内核）

- 目标：在保证 Linux 兼容性的同时，最大化发挥 Rust 的内存安全优势。
- 方法：将 unsafe Rust 严格限制在清晰可边界、尽可能小的可信计算基（TCB）内。
- 架构：采用被称为[“框内核架构”](kernel/the-framekernel-architecture.md)的设计，以模块化、可验证为目标，降低缺陷面。
- 兼容性：[Linux 兼容性说明](kernel/linux-compatibility.md)

### 2. Asterinas OSTD（操作系统开发框架）

- 目标：为安全 Rust 的 OS 开发提供最小、强大且可靠的基础库。
- 特点：类似 Rust 的 `std`，但面向内核/裸机场景；通过封装底层原语形成可验证抽象，构成最小化 TCB。
- 文档：[OSDT 总览](ostd/README.md) · [100 行安全 Rust 写内核示例](ostd/a-100-line-kernel.md)

### 3. Asterinas OSDK（命令行工具）

- 目标：提升 OS 开发的工程效率。
- 能力：新建项目、构建、运行、测试、调试、性能分析等，覆盖 OS 开发全流程。
- 文档：[OSDK 使用指南](osdk/guide/README.md) · [OSDK 命令参考](osdk/reference/README.md)

## 为什么选择 Rust 与“框内核架构”

与传统内核相比，星绽将内存安全作为第一要务：

- 使用 Rust 作为唯一实现语言，系统性地避免常见内存安全问题；
- 将 unsafe Rust 严格收敛在明确定义的最小 TCB 内；
- 通过[框内核架构](kernel/the-framekernel-architecture.md)实现安全与性能的平衡，使内核更可靠、更易于演进。

## 快速上手

准备一台安装了 Docker 的 x86-64 Linux 机器，按照以下步骤运行星绽：

1) 获取源代码

```bash
git clone https://github.com/asterinas/asterinas
```

2) 启动开发环境（Docker 容器）

```bash
docker run -it --privileged --network=host --device=/dev/kvm -v $(pwd)/asterinas:/root/asterinas asterinas/asterinas:0.15.2-20250702
```

3) 在容器内构建并运行

```bash
make build
make run
```

若一切顺利，星绽会在一个虚拟机中启动运行。

> 更多细节可参阅仓库根目录的[中文版 README](../../README_CN.md)。

## 进一步阅读

- 项目总览与指南：[The Asterinas Book](README.md)
- 内核架构与路线：
  - [框内核架构](kernel/the-framekernel-architecture.md)
  - [路线图](kernel/roadmap.md)
- OSTD 与 OSDK：
  - [OSDT 总览](ostd/README.md)
  - [OSDK 使用指南](osdk/guide/README.md)
  - [OSDK 命令参考](osdk/reference/README.md)

## 开源许可

星绽的源代码与文档主要采用
[Mozilla 公共许可证（MPL）2.0](https://github.com/asterinas/asterinas/blob/main/LICENSE-MPL)，
部分组件在更宽松的许可证下发布，详见
[.licenserc.yaml](https://github.com/asterinas/asterinas/blob/main/.licenserc.yaml)。
