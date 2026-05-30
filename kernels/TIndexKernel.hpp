/******************************************************************************
 * Copyright (c) 2015, Howard Butler (howard@hobu.co)
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

#pragma once

#include <pdal/SubcommandKernel.hpp>

namespace pdal
{

class PDAL_EXPORT TIndexKernel : public SubcommandKernel
{
public:
    std::string getName() const override;
    TIndexKernel();

private:
    void addSubSwitches(ProgramArgs& args,
                        const std::string& subcommand) override;
    void validateSwitches(ProgramArgs& args) override;
    int execute() override;
    StringList subcommands() const override;

    std::string m_idxFilename;
    std::string m_filespec;
    std::string m_listfile;
    std::string m_layerName;
    std::string m_driverName;
    std::string m_tileIndexColumnName;
    std::string m_wkt;
    StringList m_lcOptions;
    BOX2D m_bounds;
    bool m_absPath;
    std::string m_prefix;
    int m_threads;
    bool m_doSmooth;
    int32_t m_density;
    double m_edgeLength;
    uint32_t m_sampleSize;
    std::string m_boundaryExpr;

    std::string m_tgtSrsString;
    std::string m_assignSrsString;
    bool m_fastBoundary;
    bool m_usestdin;
    bool m_overrideASrs;
    bool m_skipMultiSrs;
};

} // namespace pdal
