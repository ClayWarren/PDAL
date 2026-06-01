/******************************************************************************
 * Copyright (c) 2011, Michael P. Gerlek (mpg@flaxen.com)
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

#include "DeltaKernel.hpp"

#include <pdal_capi.h>

#include <vector>

namespace pdal
{

static StaticPluginInfo const s_info{"kernels.delta",
                                     "Delta Kernel [DEPRECATED]",
                                     "https://pdal.org/apps/delta.html"};

CREATE_STATIC_KERNEL(DeltaKernel, s_info)

std::string DeltaKernel::getName() const
{
    return s_info.name;
}

DeltaKernel::DeltaKernel() : m_detail(false), m_allDims(false) {}

void DeltaKernel::addSwitches(ProgramArgs& args)
{
    Arg& src = args.add("source", "source file name", m_sourceFile);
    src.setPositional();
    Arg& candidate =
        args.add("candidate", "candidate file name", m_candidateFile);
    candidate.setPositional();
    args.add("detail", "Output deltas per-point", m_detail);
    args.add("alldims", "Compute diffs for all dimensions (not just X,Y,Z)",
             m_allDims);
}

int DeltaKernel::execute()
{
    StringList args;
    args.push_back(m_sourceFile);
    args.push_back(m_candidateFile);
    if (m_detail)
        args.push_back("--detail");
    if (m_allDims)
        args.push_back("--alldims");

    std::vector<const char*> argv;
    argv.reserve(args.size());
    for (const std::string& arg : args)
        argv.push_back(arg.c_str());

    return pdal_rust_kernel_run("delta", static_cast<int>(argv.size()),
                                argv.data());
}

} // namespace pdal
