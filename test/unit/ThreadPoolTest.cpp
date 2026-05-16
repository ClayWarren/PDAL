/******************************************************************************
 * Copyright (c) 2026, Hobu Inc.
 *
 * All rights reserved.
 *
 * Redistribution and use in source and binary forms, with or without
 * modification, are permitted provided that the following
 * conditions are met:
 *
 *     * Redistributions of source code must retain the above copyright
 *       notice, this list of conditions and the following disclaimer.
 *     * Redistributions in binary form must reproduce the above copyright
 *       notice, this list of conditions and the following disclaimer in
 *       the documentation and/or other materials provided
 *       with the distribution.
 *     * Neither the name of Hobu, Inc. or Flaxen Geo Consulting nor the
 *       names of its contributors may be used to endorse or promote
 *       products derived from this software without specific prior
 *       written permission.
 *
 * THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS
 * "AS IS" AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT
 * LIMITED TO, THE IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS
 * FOR A PARTICULAR PURPOSE ARE DISCLAIMED. IN NO EVENT SHALL THE
 * COPYRIGHT OWNER OR CONTRIBUTORS BE LIABLE FOR ANY DIRECT, INDIRECT,
 * INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES (INCLUDING,
 * BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES; LOSS
 * OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED
 * AND ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY,
 * OR TORT (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT
 * OF THE USE OF THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY
 * OF SUCH DAMAGE.
 ****************************************************************************/

#include <pdal/pdal_test_main.hpp>

#include <atomic>

#include <pdal/util/ThreadPool.hpp>

namespace pdal
{

TEST(ThreadPoolTest, await_runs_enqueued_tasks)
{
    ThreadPool pool(2);
    std::atomic<int> count(0);

    for (int i = 0; i < 12; ++i)
        pool.add([&count]() { ++count; });

    pool.await();
    EXPECT_EQ(count.load(), 12);
}

TEST(ThreadPoolTest, stop_and_restart)
{
    ThreadPool pool(2);
    std::atomic<int> count(0);

    pool.stop();
    EXPECT_THROW(pool.add([]() {}), pdal_error);

    pool.go();
    pool.add([&count]() { ++count; });
    pool.await();
    EXPECT_EQ(count.load(), 1);

    pool.resize(1);
    EXPECT_EQ(pool.numThreads(), 1u);
    pool.add([&count]() { ++count; });
    pool.await();
    EXPECT_EQ(count.load(), 2);
}

} // namespace pdal
