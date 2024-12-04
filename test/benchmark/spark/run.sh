#!/bin/sh

# SPDX-License-Identifier: MPL-2.0

set -e

export JAVA_HOME=/usr/local/jre/
export PATH=$JAVA_HOME/bin:$PATH
export SPARK_HOME=/usr/local/spark/
export SPARK_LOCAL_IP=127.0.0.1
META_SPACE=1g

ulimit -c 65536
# "${SPARK_HOME}"/bin/spark-submit --class org.apache.spark.repl.Main --name "Spark shell"
# cd /usr/local/
# java NativeHeapAllocationExample

# rm $SPARK_HOME/jars/slf4j-reload4j-1.7.35.jar

java \
    -XX:+UseCompressedOops \
    -XX:MaxMetaspaceSize=$META_SPACE \
    -XX:ActiveProcessorCount=1 \
    -Divy.home="/tmp/.ivy" \
    -Dos.name="Linux" \
    -cp "$SPARK_HOME/conf/:$SPARK_HOME/jars/*" \
    -Xmx10g org.apache.spark.deploy.SparkSubmit \
    --jars $SPARK_HOME/examples/jars/spark-examples_2.12-3.1.3.jar,$SPARK_HOME/examples/jars/scopt_2.12-3.7.1.jar \
    --class org.apache.spark.examples.SparkPi spark-internal
# java \
#     -XX:+UseCompressedOops \
#     -XX:MaxMetaspaceSize=$META_SPACE \
#     -XX:ActiveProcessorCount=1 \
#     -Divy.home="/tmp/.ivy" \
#     -Dos.name="Linux" \
#     -cp "$SPARK_HOME/conf/:$SPARK_HOME/jars/*" \
#     -Xmx10g org.apache.spark.deploy.SparkSubmit \
#     --jars $SPARK_HOME/examples/jars/spark-examples_2.12-3.1.3.jar,$SPARK_HOME/examples/jars/scopt_2.12-3.7.1.jar \
#     --class org.apache.spark.examples.SparkPi \
#     --executor-memory 4g \
#     --num-executors 4 \
#     --conf spark.default.parallelism=100 \
#     spark-internal