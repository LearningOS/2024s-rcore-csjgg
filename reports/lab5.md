# Lab5

## 编程作业

感觉难度在于理解

- 在加锁之前要运行一边算法,保证不会死锁
  - 对现在要加锁的task和锁,need要加1
  - 比较的过程中不是`Need[i,j] ≤ Work[j];`而是`对所有j: Need[i,j] ≤ Work[j];`

- 加锁成功后要对need - 1, alloc+1, availabe -1
- 解锁要对alloc-1,available+1
- 其余的过程和描述基本一致.

设计上,不要设计成矩阵,不好维护

- 每个task有两个vec, 一个是need 一个是alloc
- 每个process有一个vec,代表锁资源
- 初始化task时要根据process中锁列表的长度初始化alloc和need
- 增加锁的时候也要增加task中alloc和need的长度
- 增加锁的时候当然也要增加process中的available的长度

对于信号量也是一样

系统调用开启检测只需要在pcb中增加一个flag即可,同时需要在做检测前先看一下flag,是false就不检测了

## 问答作业

1. 在我们的多线程实现中，当主线程 (即 0 号线程) 退出时，视为整个进程退出， 此时需要结束该进程管理的所有线程并回收其资源。 - 需要回收的资源有哪些？ - 其他线程的 TaskControlBlock 可能在哪些位置被引用，分别是否需要回收，为什么？

> 回收的资源包括线程和进程的资源,所有线程的TCB都要回收,PCB也要进行回收,其中的锁列表,文件表,系统内存资源等都要进行回收
>
> 其他线程的TCB可能在线程调度器或者其他线程的wait队列中引用, 是需要回收的. 为了避免资源的浪费和类似死锁的问题.

2. 对比以下两种 `Mutex.unlock` 的实现，二者有什么区别？这些区别可能会导致什么问题？

```rust
 1impl Mutex for Mutex1 {
 2    fn unlock(&self) {
 3        let mut mutex_inner = self.inner.exclusive_access();
 4        assert!(mutex_inner.locked);
 5        mutex_inner.locked = false;
 6        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
 7            add_task(waking_task);
 8        }
 9    }
10}
11
12impl Mutex for Mutex2 {
13    fn unlock(&self) {
14        let mut mutex_inner = self.inner.exclusive_access();
15        assert!(mutex_inner.locked);
16        if let Some(waking_task) = mutex_inner.wait_queue.pop_front() {
17            add_task(waking_task);
18        } else {
19            mutex_inner.locked = false;
20        }
21    }
22}
```

>  将mutex中locked标志为false的时机不同
>
> mutex1感觉会有问题,如果其他线程尝试获取锁,可能会在add_task之前将锁获取,结果新的task add到调度器中也不会执行,有资源浪费
>
> mutex2更安全一些,直接确定了下一个运行的task一定是取出来的这个

## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > 无

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > 实验文档

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。