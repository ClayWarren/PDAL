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

#include "pdal/pdal_features.hpp"

#include "PipelineKernel.hpp"

#include <pdal_capi.h>

#include <sstream>
#include <vector>

namespace pdal
{

static StaticPluginInfo const s_info{"kernels.pipeline", "Pipeline Kernel",
                                     "https://pdal.org/apps/pipeline.html"};

CREATE_STATIC_KERNEL(PipelineKernel, s_info)

std::string PipelineKernel::getName() const
{
    return s_info.name;
}

PipelineKernel::PipelineKernel() : m_validate(false) {}

void PipelineKernel::validateSwitches(ProgramArgs& args)
{
    if (m_usestdin)
        m_inputFile = "STDIN";

    if (m_inputFile.empty())
        throw pdal_error("Input filename required.");

    if (m_stream && m_noStream)
        throw pdal_error("Can't execute with 'stream' and 'nostream' options");
    if (m_stream)
        m_mode = ExecMode::Stream;
    else if (m_noStream)
        m_mode = ExecMode::Standard;
    else
        m_mode = ExecMode::PreferStream;
}

bool PipelineKernel::isStagePrefix(const std::string& stage)
{
    return Kernel::isStagePrefix(stage) || stage == "stage";
}

void PipelineKernel::addSwitches(ProgramArgs& args)
{
    args.add("input,i", "Input filename", m_inputFile).setOptionalPositional();

    args.add("pipeline-serialization", "Output file for pipeline serialization",
             m_pipelineFile);
    args.add("validate",
             "Validate the pipeline (including serialization), "
             "but do not write points",
             m_validate);
    args.add("progress",
             "Name of file or FIFO to which stages should write progress "
             "information.  The file/FIFO must exist.  PDAL will not create "
             "the progress file.",
             m_progressFile);
    args.add("pointcloudschema", "dump PointCloudSchema XML output",
             m_PointCloudSchemaOutput)
        .setHidden();
    args.add("stdin,s", "Read pipeline from standard input", m_usestdin);
    args.add("stream", "Run in stream mode.  Error if not streamable.",
             m_stream);
    args.add("nostream", "Run in standard mode.", m_noStream);
    args.add("metadata", "Metadata filename", m_metadataFile);
    args.add("dims", "Dimensions to be stored", m_dimNames);
}

int PipelineKernel::execute()
{
    StringList args;
    if (m_usestdin)
        args.push_back("--stdin");
    else
        args.push_back(m_inputFile);
    if (m_validate)
        args.push_back("--validate");
    if (m_pipelineFile.size())
    {
        args.push_back("--pipeline-serialization");
        args.push_back(m_pipelineFile);
    }
    if (m_metadataFile.size())
    {
        args.push_back("--metadata");
        args.push_back(m_metadataFile);
    }
    if (m_stream)
        args.push_back("--stream");
    if (m_noStream)
        args.push_back("--nostream");
    if (m_dimNames.size())
    {
        std::ostringstream dims;
        for (size_t i = 0; i < m_dimNames.size(); ++i)
        {
            if (i)
                dims << ',';
            dims << m_dimNames[i];
        }
        args.push_back("--dims");
        args.push_back(dims.str());
    }
    if (m_progressFile.size())
    {
        args.push_back("--progress");
        args.push_back(m_progressFile);
    }
    if (m_PointCloudSchemaOutput.size())
    {
        args.push_back("--pointcloudschema");
        args.push_back(m_PointCloudSchemaOutput);
    }

    std::vector<const char*> argv;
    argv.reserve(args.size());
    for (const std::string& arg : args)
        argv.push_back(arg.c_str());

    return pdal_rust_kernel_run("pipeline", static_cast<int>(argv.size()),
                                argv.data());
}

} // namespace pdal
