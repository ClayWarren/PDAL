/******************************************************************************
 * Copyright (c) 2011, Michael P. Gerlek (mpg@flaxen.com)
 * Copyright (c) 2015, Bradley J Chambers (brad.chambers@gmail.com)
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

#include "TranslateKernel.hpp"

#include <rust/pdal-capi/include/pdal_capi.h>

#include <sstream>
#include <vector>

namespace pdal
{

static StaticPluginInfo const s_info{
    "kernels.translate",
    "The Translate kernel allows users to construct a pipeline "
    "consisting of a reader, a writer, and N filter stages. "
    "Any supported stage type can be specified from the command "
    "line, reducing the need to create custom kernels for every "
    "combination.",
    "https://pdal.org/apps/translate.html"};

CREATE_STATIC_KERNEL(TranslateKernel, s_info)

std::string TranslateKernel::getName() const
{
    return s_info.name;
}

TranslateKernel::TranslateKernel() {}

void TranslateKernel::addSwitches(ProgramArgs& args)
{
    args.add("input,i", "Input filename", m_inputFile).setPositional();
    args.add("output,o", "Output filename", m_outputFile).setPositional();
    args.add("filter,f", "Filter type", m_filterType).setOptionalPositional();
    args.add("json", "PDAL pipeline from which to extract filters.",
             m_filterJSON);
    args.add("pipeline,p", "Pipeline output", m_pipelineOutputFile);
    args.add("metadata,m", "Dump metadata output to the specified file",
             m_metadataFile);
    args.add("reader,r", "Reader type", m_readerType);
    args.add("writer,w", "Writer type", m_writerType);
    args.add("nostream", "Run in standard mode", m_noStream);
    args.add("stream", "Run in stream mode.  Error if not possible.", m_stream);
    args.add("dims", "Dimensions to store", m_dimNames);
    args.add("overwrite", "Overwrite existing input", m_overwriteInput, false);
}

void TranslateKernel::validateSwitches(ProgramArgs&)
{
    if (m_stream && m_noStream)
        throw pdal_error("Can't specify both 'stream' and 'nostream' options.");

    if (m_stream)
        m_mode = ExecMode::Stream;
    else if (m_noStream)
        m_mode = ExecMode::Standard;
    else
        m_mode = ExecMode::PreferStream;

    if (Utils::iequals(m_inputFile, m_outputFile) && !m_overwriteInput)
        throw pdal_error("Input and output filenames are equal and no "
                         "--overwrite option was provided!");
}

int TranslateKernel::execute()
{
    if (m_filterJSON.size() && m_filterType.size())
        throw pdal_error("Cannot set both --filter options and --json options");

    StringList args;
    args.push_back(m_inputFile);
    args.push_back(m_outputFile);
    for (const std::string& filter : m_filterType)
        args.push_back(filter);
    if (!m_filterJSON.empty())
    {
        args.push_back("--json");
        args.push_back(m_filterJSON);
    }
    if (!m_pipelineOutputFile.empty())
    {
        args.push_back("--pipeline");
        args.push_back(m_pipelineOutputFile);
    }
    if (!m_metadataFile.empty())
    {
        args.push_back("--metadata");
        args.push_back(m_metadataFile);
    }
    if (!m_readerType.empty())
    {
        args.push_back("--reader");
        args.push_back(m_readerType);
    }
    if (!m_writerType.empty())
    {
        args.push_back("--writer");
        args.push_back(m_writerType);
    }
    if (m_noStream)
        args.push_back("--nostream");
    if (m_stream)
        args.push_back("--stream");
    if (m_overwriteInput)
        args.push_back("--overwrite");
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

    std::vector<const char*> argv;
    argv.reserve(args.size());
    for (const std::string& arg : args)
        argv.push_back(arg.c_str());

    return pdal_rust_kernel_run("translate", static_cast<int>(argv.size()),
                                argv.data());
}

} // namespace pdal
