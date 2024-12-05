#include <stdio.h>
#include <stdlib.h>
#include <unistd.h>
#include <sys/socket.h>
#include <string.h>
#include <sys/wait.h>
#include <signal.h>
#include <errno.h>

void handle_sigpipe(int sig)
{
	// Do nothing, just catch the signal
}

int main()
{
	int sv[2]; // Socket pair
	char buffer[100];

	// Ignore SIGPIPE signal
	signal(SIGPIPE, handle_sigpipe);

	// Create a socket pair with SOCK_CLOEXEC flag
	if (socketpair(AF_UNIX, SOCK_STREAM | SOCK_CLOEXEC, 0, sv) == -1) {
		perror("socketpair");
		exit(EXIT_FAILURE);
	}

	// Fork a new process
	pid_t pid = fork();
	if (pid == -1) {
		perror("fork");
		exit(EXIT_FAILURE);
	}

	if (pid == 0) { // Child process
		// Close the parent's end of the socket pair
		if (close(sv[0]) == -1) {
			perror("close");
			exit(EXIT_FAILURE);
		}

		// Send a message through the child's end of the socket pair
		const char *message = "Hello from child!";
		if (write(sv[1], message, strlen(message)) == -1) {
			if (errno == EPIPE) {
				fprintf(stderr,
					"EPIPE error: The other end of the socket is closed.\n");
			} else {
				perror("write");
			}
			exit(EXIT_FAILURE);
		}

		// Close the child's end of the socket pair
		if (close(sv[1]) == -1) {
			perror("close");
			exit(EXIT_FAILURE);
		}

		exit(EXIT_SUCCESS);
	} else { // Parent process
		// Close the child's end of the socket pair
		if (close(sv[1]) == -1) {
			perror("close");
			exit(EXIT_FAILURE);
		}

		// Receive the message through the parent's end of the socket pair
		if (read(sv[0], buffer, sizeof(buffer)) == -1) {
			perror("read");
			exit(EXIT_FAILURE);
		}

		// Print the received message
		printf("Received message: %s\n", buffer);

		// Close the parent's end of the socket pair
		if (close(sv[0]) == -1) {
			perror("close");
			exit(EXIT_FAILURE);
		}

		// Wait for the child process to finish
		int status;
		if (waitpid(pid, &status, 0) == -1) {
			perror("waitpid");
			exit(EXIT_FAILURE);
		}
	}

	return 0;
}