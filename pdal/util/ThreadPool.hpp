/******************************************************************************
 * Copyright (c) 2018, Connor Manning
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
 *     * Neither the name of the Martin Isenburg or Iowa Department
 *       of Natural Resources nor the names of its contributors may be
 *       used to endorse or promote products derived from this software
 *       without specific prior written permission.
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

#pragma once

#include <algorithm>
#include <cassert>
#include <functional>

#include <pdal/pdal_types.hpp>
#include <pdal_capi.h>

namespace pdal
{

class ThreadPool
{
public:
    // After numThreads tasks are actively running, and queueSize tasks have
    // been enqueued to wait for an available worker thread, subsequent calls
    // to Pool::add will block until an enqueued task has been popped from the
    // queue.
    PDAL_EXPORT ThreadPool(std::size_t numThreads, int64_t queueSize = -1,
                           bool verbose = true)
    {
        assert(queueSize != 0);
        m_pool = pdal_thread_pool_create(std::max<std::size_t>(numThreads, 1),
                                         queueSize);
    }

    PDAL_EXPORT ~ThreadPool()
    {
        pdal_thread_pool_destroy(m_pool);
    }

    ThreadPool(const ThreadPool& other) = delete;
    ThreadPool& operator=(const ThreadPool& other) = delete;

    // Start worker threads.
    PDAL_EXPORT void go();

    // Disallow the addition of new tasks and wait for all currently running
    // tasks to complete.
    PDAL_EXPORT void join()
    {
        pdal_thread_pool_join(m_pool);
    }

    // join() and empty the queue of tasks that may have been waiting to run.
    PDAL_EXPORT void stop()
    {
        pdal_thread_pool_stop(m_pool);
    }

    // Empty the queue of tasks that may have been waiting to run.
    PDAL_EXPORT void clearTasks()
    {
        pdal_thread_pool_clear_tasks(m_pool);
    }

    // Wait for all current tasks to complete.  As opposed to join, tasks may
    // continue to be added while a thread is await()-ing the queue to empty.
    PDAL_EXPORT void await()
    {
        pdal_thread_pool_await(m_pool);
    }

    // Join and restart.
    PDAL_EXPORT void cycle()
    {
        join();
        go();
    }

    // Change the number of threads.  Current threads will be joined.
    PDAL_EXPORT void resize(const std::size_t numThreads)
    {
        pdal_thread_pool_resize(m_pool, numThreads);
    }

    // Add a threaded task, blocking until a thread is available.  If join() is
    // called, add() may not be called again until go() is called and completes.
    PDAL_EXPORT void add(std::function<void()> task)
    {
        auto* heapTask = new std::function<void()>(std::move(task));
        if (!pdal_thread_pool_add(m_pool, heapTask, runTask, dropTask))
        {
            delete heapTask;
            throw pdal_error("Attempted to add a task to a stopped ThreadPool");
        }
    }

    PDAL_EXPORT std::size_t size() const
    {
        return numThreads();
    }

    PDAL_EXPORT std::size_t numThreads() const
    {
        return pdal_thread_pool_num_threads(m_pool);
    }

private:
    static void runTask(void* data)
    {
        auto* task = static_cast<std::function<void()>*>(data);
        (*task)();
        delete task;
    }

    static void dropTask(void* data)
    {
        delete static_cast<std::function<void()>*>(data);
    }

    pdal_thread_pool_t* m_pool = nullptr;
};

} // namespace pdal
