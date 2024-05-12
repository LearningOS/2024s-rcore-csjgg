# Lab 3

## 编程作业

### Spawn

- 为TCB新增加一个spawn的函数

  - 读取elf数据,获得memory set 和 entry point等,就像TCB::new一样
  - 分配内核栈
  - 分配完整TCB
    - 要设置父亲

  - 设置trapcx
  - 要将父亲的子类中加入新的task

- 在syscall中完成类似fork的工作,设置返回值和加入新任务.

### stride

- 为TCB增加优先级和stride
- 在config中设置big_stride
- 在suspend_current_and_run_next增加更新stride的代码
- 在run next的时候选择最小stride的app-> 在idle中-> 在fetch函数中找到最小的,删除并返回

对于系统调用:

- 新增一个更新por的函数
- 检查优先级,不能小于等于1
- 更新成功返回设置的优先级

## 问答作业

- 实际情况是轮到 p1 执行吗？为什么？

> 不是,发生了溢出,p2的stride又小于p1了

- 为什么？尝试简单说明（不要求严格证明）。

> 通过设置优先级的最小值为2或更高，可以确保任何进程的步幅不会超过`BigStride / 2`,2次调度之间步幅的差异（`STRIDE_MAX - STRIDE_MIN`）也不会超过`BigStride / 2`。

- 代码

```rust
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        const MAX_STRIDE: u64 = 255;
        let half_max = MAX_STRIDE / 2;

        // Calculate the distance assuming wrap around at MAX_STRIDE + 1
        let distance = |x: u64| {
            if x <= half_max {
                half_max - x
            } else {
                MAX_STRIDE + 1 - x + half_max
            }
        };

        let self_distance = distance(self.0);
        let other_distance = distance(other.0);

        // Compare based on distance from the theoretical midpoint
        self_distance.partial_cmp(&other_distance)
    }
}

impl PartialEq for Stride {
    fn eq(&self, _other: &Self) -> bool {
        false
    }
}
```

## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > 无

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > 实验文档

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。

