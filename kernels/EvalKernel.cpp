/******************************************************************************
 * Copyright (c) 2020, Bradley J Chambers (brad.chambers@gmail.com)
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

#include "EvalKernel.hpp"

#include <rust/pdal-capi/include/pdal_capi.h>

#include <vector>

namespace pdal
{

using namespace Dimension;

static StaticPluginInfo const s_info{
    "kernels.eval", "Eval Kernel [DEPRECATED]",
    "https://pdal.org/kernels/kernels.eval.html"};

CREATE_STATIC_KERNEL(EvalKernel, s_info)

std::string EvalKernel::getName() const
{
    return s_info.name;
}

void EvalKernel::addSwitches(ProgramArgs& args)
{
    args.add("predicted", "Point cloud filename containing predicted labels",
             m_predictedFile)
        .setPositional();
    args.add("truth", "Point cloud filename containing truth labels",
             m_truthFile)
        .setPositional();
    args.add("labels",
             "Comma-separated list of classification labels to evaluate",
             m_labelStrList);
    args.add("prediction_dim", "Dimension containing predicted labels",
             m_predictedDimName, "Classification");
    args.add("truth_dim", "Dimension containing truth labels", m_truthDimName,
             "Classification");
}

void EvalKernel::validateSwitches(ProgramArgs& args)
{
    if (m_labelStrList.empty())
        throw pdal_error(
            "Must specify comma-separated list of labels to evaluate.");
}

int EvalKernel::execute()
{
    StringList args;
    args.push_back(m_predictedFile);
    args.push_back(m_truthFile);

    std::string labels;
    for (size_t i = 0; i < m_labelStrList.size(); ++i)
    {
        if (i)
            labels += ",";
        labels += m_labelStrList[i];
    }
    args.push_back("--labels");
    args.push_back(labels);
    args.push_back("--prediction_dim");
    args.push_back(m_predictedDimName);
    args.push_back("--truth_dim");
    args.push_back(m_truthDimName);

    std::vector<const char*> argv;
    argv.reserve(args.size());
    for (const std::string& arg : args)
        argv.push_back(arg.c_str());

    return pdal_rust_kernel_run("eval", static_cast<int>(argv.size()),
                                argv.data());
}

} // namespace pdal
