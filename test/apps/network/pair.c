#define _GNU_SOURCE
#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/types.h>
#include <sys/socket.h>
#include <sys/wait.h>
#include <string.h>

// Function to create a socketpair of type AF_LOCAL for communication between parent and child processes.
int new_sock_pair(int fds[2]) {
    if (socketpair(AF_LOCAL, SOCK_STREAM, 0, fds) == -1) {
        perror("socketpair");
        return -1;
    }
    return 0;
}

int main() {
    int fds[2];
    char buffer[1024];
    char message[1024];
    pid_t pid;

    // Create a socketpair
    if (new_sock_pair(fds) == -1) {
        fprintf(stderr, "Failed to create socketpair\n");
        return 1;
    }

    // Fork the process to create a child process
    pid = fork();
    if (pid < 0) {
        perror("fork");
        return 1;
    }

    if (pid == 0) {
        // Child process
        close(fds[0]); // Close the parent's end of the socketpair

        // Child process receives a message from the parent
        int n = read(fds[1], buffer, sizeof(buffer));
        if (n < 0) {
            perror("Child failed to read");
            exit(1);
        }
        printf("Child received: %s\n", buffer);

        // Child process sends a message back to the parent
        strcpy(message, "Hello from child");
        if (write(fds[1], message, strlen(message)) < 0) {
            perror("Child failed to write");
            exit(1);
        }
        printf("Child sent: %s\n", message);

        close(fds[1]);
        exit(0);
    } else {
        // Parent process
        close(fds[1]); // Close the child's end of the socketpair

        // Parent process sends a message
        strcpy(message, "Hello from parent");
        if (write(fds[0], message, strlen(message)) < 0) {
            perror("Parent failed to write");
            return 1;
        }
        printf("Parent sent: %s\n", message);

        // Parent process receives a message from the child
        int n = read(fds[0], buffer, sizeof(buffer));
        if (n < 0) {
            perror("Parent failed to read");
            return 1;
        }
        printf("Parent received: %s\n", buffer);

        // Wait for the child process to complete
        wait(NULL);
        close(fds[0]);
    }

    return 0;
}