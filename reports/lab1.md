# 实验报告

## 编程作业

思路如下

- 在TCB中增加记录系统调用次数和初次调度时间的成员
- 当发生syscall时,修改TCB, 增加对应系统调用的调用次数
- 当task被初次调度时, 修改TCB,设置时间为初次调度时间
- 调用task info时, 读取TCB中的信息并返回

对应的, 需要修改的主要是syscall中的部分和TaskManager的部分

增加了一些相关的函数.

## 问答题

1.  **正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 [三个 bad 测例 (ch2b_bad_*.rs)](https://github.com/LearningOS/rCore-Tutorial-Test-2024S/tree/master/src/bin) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。**
      - bad_address 会访问错误的地址,出发pagefault,会被traphandler 处理, 退出当前task并运行下一个
      
      - bad_instructions 会运行一个S特权命令,但是由于权限不符合,所以也会报错,被traphandler捕获`Exception::IllegalInstruction`然后run next app
      
      - bad_register会访问sstatus 这个S态寄存器不能被用户态程序访问,所以也会触发trap`Exception::IllegalInstruction`

1. **深入理解 [trap.S](https://github.com/LearningOS/rCore-Tutorial-Code-2024S/blob/ch3/os/src/trap/trap.S) 中两个函数 `__alltraps` 和 `__restore` 的作用，并回答如下问题:**

   1. **L40：刚进入 `__restore` 时，`a0` 代表了什么值。请指出 `__restore` 的两种使用情景。**

      a0 代表的是调用__restore传入的第一个参数, 具体应该是内核中保存的trapcontext的地址

   2. **L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。**

      ```
      ld t0, 32*8(sp)
      ld t1, 33*8(sp)
      ld t2, 2*8(sp)
      csrw sstatus, t0
      csrw sepc, t1
      csrw sscratch, t2
      ```

      主要是恢复`sstatus` `sepc` 和`sscratch` 

      - `sstatus`的目的是恢复权限为用户态 
      - `sepc`的作用是保存tarp发生是程序的指令地址,现在恢复过程当然要恢复,这样才能让程序回到之前的位置
      - `sscratch`是用于暂存stack pointer的, 它会在后续被更换成内核栈中trap context的地址,而把用户sp的地址设置回sp去

   3. **L50-L56：为何跳过了 `x2` 和 `x4`？**

      ```
      ld x1, 1*8(sp)
      ld x3, 3*8(sp)
      .set n, 5
      .rept 27
         LOAD_GP %n
         .set n, n+1
      .endr
      ```

      引用原话`如 x0 被硬编码为 0 ，它自然不会有变化；还有 tp(x4) 寄存器，除非我们手动出于一些特殊用途使用它，否则一般也不会被用到。`

   4. **L60：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？**

      ```
      csrrw sp, sscratch, sp
      ```

      `sscratch`中的值被保存为内核栈中trapcontext的地址,方便下一次使用

      `sp`的地址被恢复为原来sp的地址,保证用户程序正确运行

   5. **`__restore`：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？**

      `sret` 这是一个特权指令,用于从高的权限降低到低权限

   6. **L13：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？**

      ```
      csrrw sp, sscratch, sp
      ```

      `sscratch`中的值被保存为用户`sp`的值,用于后续的保存和最后恢复应用时的恢复

      `sp`中的值被替换成之前进行`__restore`是保存的trapcontext的地址,方便下一步使用sp进行保存寄存器

   7. **从 U 态进入 S 态是哪一条指令发生的？**

      `ecall`

## 荣誉准则

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > 无

2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > 实验文档

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。