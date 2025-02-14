#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <signal.h>
#include <sys/signalfd.h>
#include <sys/syscall.h>

// 获取当前线程ID（Linux特有方式）
static pid_t gettid() {
    return syscall(SYS_gettid);
}

int main(int argc, char *argv[]) {
    // 选择要测试的信号，默认为SIGUSR1(10)
    int test_signal = SIGUSR1;
    if (argc > 1) {
        test_signal = atoi(argv[1]);
    }

    // 步骤1: 创建信号掩码
    sigset_t mask;
    sigemptyset(&mask);
    if (sigaddset(&mask, test_signal) == -1) {
        perror("sigaddset");
        exit(EXIT_FAILURE);
    }

    // 步骤2: 创建signalfd
    int sfd = signalfd(-1, &mask, SFD_CLOEXEC);
    if (sfd == -1) {
        perror("signalfd");
        exit(EXIT_FAILURE);
    }

    // 步骤3: 阻塞信号
    if (sigprocmask(SIG_BLOCK, &mask, NULL) == -1) {
        perror("sigprocmask");
        close(sfd);
        exit(EXIT_FAILURE);
    }

    // 步骤4: 发送信号给自己
    pid_t pid = getpid();
    pid_t tid = gettid();
    if (syscall(SYS_tgkill, pid, tid, test_signal) == -1) {
        perror("tgkill");
        close(sfd);
        exit(EXIT_FAILURE);
    }

    // 步骤5: 读取信号信息
    struct signalfd_siginfo fdsi;
    ssize_t bytes = read(sfd, &fdsi, sizeof(fdsi));
    if (bytes != sizeof(fdsi)) {
        fprintf(stderr, "Read error: expected %zu, got %zd\n",
                sizeof(fdsi), bytes);
        close(sfd);
        exit(EXIT_FAILURE);
    }

    // 验证接收的信号
    if (fdsi.ssi_signo == test_signal) {
        printf("Success: Received signal %d\n", test_signal);
    } else {
        fprintf(stderr, "Error: Expected signal %d, got %d\n",
                test_signal, fdsi.ssi_signo);
    }

    // 清理
    close(sfd);
    return EXIT_SUCCESS;
}
