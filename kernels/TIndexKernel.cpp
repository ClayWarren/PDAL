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
 *       the documentation and/or other materials provided with the
 *       distribution.
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

#include "TIndexKernel.hpp"

#include <pdal_capi.h>

#include <sstream>
#include <vector>

namespace pdal
{

static StaticPluginInfo const s_info{"kernels.tindex", "TIndex Kernel",
                                     "https://pdal.org/apps/tindex.html"};

CREATE_STATIC_KERNEL(TIndexKernel, s_info)

namespace
{

void addOption(StringList& args, const std::string& name,
               const std::string& value)
{
    if (!value.empty())
    {
        args.push_back(name);
        args.push_back(value);
    }
}

void addBoolOption(StringList& args, const std::string& name, bool value)
{
    if (value)
        args.push_back(name + "=true");
}

void addFlag(StringList& args, const std::string& name, bool value)
{
    if (value)
        args.push_back(name);
}

template <typename T>
void addScalarOption(StringList& args, const std::string& name, T value)
{
    std::ostringstream out;
    out << value;
    addOption(args, name, out.str());
}

} // namespace

std::string TIndexKernel::getName() const
{
    return s_info.name;
}

TIndexKernel::TIndexKernel() : SubcommandKernel(), m_overrideASrs(false) {}

StringList TIndexKernel::subcommands() const
{
    return {"create", "merge"};
}

void TIndexKernel::addSubSwitches(ProgramArgs& args,
                                  const std::string& subcommand)
{
    if (subcommand == "create")
    {
        args.add("tindex", "OGR-readable/writeable tile index output",
                 m_idxFilename)
            .setPositional();
        args.add("glob", "Pattern of files to index", m_filespec)
            .setOptionalPositional();
        args.addSynonym("glob", "filespec");
        args.add("filelist", "Text file containing list of files to index",
                 m_listfile);
        args.add("fast_boundary", "Use extent instead of exact boundary",
                 m_fastBoundary);
        args.add("lyr_name", "OGR layer name to write into datasource",
                 m_layerName);
        args.add("tindex_name", "Tile index column name", m_tileIndexColumnName,
                 "location");
        args.add("ogrdriver,f", "OGR driver name to use ", m_driverName,
                 "ESRI Shapefile");
        args.add("lco", "Driver-specific NAME=VALUE OGR layer creation options",
                 m_lcOptions);
        args.add("t_srs", "Target SRS of tile index", m_tgtSrsString,
                 "EPSG:4326");
        args.add("a_srs", "Assign SRS of tile with no SRS to this value",
                 m_assignSrsString, "EPSG:4326");
        args.add("write_absolute_path",
                 "Write absolute rather than relative file paths", m_absPath);
        args.add("stdin,s", "Read filespec pattern from standard input",
                 m_usestdin);
        args.add("path_prefix",
                 "Prefix to be added to file paths when writing "
                 "output",
                 m_prefix);
        args.add("threads",
                 "Number of threads to use for file boundary creation",
                 m_threads, 1);
        args.addSynonym("threads", "requests");
        args.add("skip_different_srs",
                 "Reject files to be indexed with "
                 "different SRS values",
                 m_skipMultiSrs);
        args.add("simplify", "Simplify the file's exact boundary", m_doSmooth,
                 true);
        args.addSynonym("simplify", "smooth");
        args.add("threshold",
                 "Number of points a cell must contain to be "
                 "declared positive space, when creating exact boundaries",
                 m_density, 15);
        args.add("resolution",
                 "cell edge length to be used when creating exact "
                 "boundaries",
                 m_edgeLength);
        args.addSynonym("resolution", "edge_length");
        args.add("sample_size",
                 "Sample size for auto-edge length calculation in "
                 "internal hexbin filter (exact boundary)",
                 m_sampleSize, 5000U);
        args.add("where",
                 "Expression describing points to be processed for exact "
                 "boundary creation",
                 m_boundaryExpr);
    }
    else if (subcommand == "merge")
    {
        args.add("tindex", "OGR-readable/writeable tile index output",
                 m_idxFilename)
            .setPositional();
        args.add("filespec", "Output filename", m_filespec).setPositional();
        args.add("lyr_name", "OGR layer name to write into datasource",
                 m_layerName);
        args.add("tindex_name", "Tile index column name", m_tileIndexColumnName,
                 "location");
        args.add("ogrdriver,f", "OGR driver name to use ", m_driverName,
                 "ESRI Shapefile");
        args.add("bounds", "Extent (in XYZ) to clip output to", m_bounds);
        args.add("polygon", "Well-known text of polygon to clip output", m_wkt);
        args.add("t_srs", "Spatial reference of the clipping geometry",
                 m_tgtSrsString, "EPSG:4326");
    }
}

void TIndexKernel::validateSwitches(ProgramArgs& args)
{
    if (m_subcommand == "merge")
    {
        if (!m_wkt.empty() && !m_bounds.empty())
            throw pdal_error("Can't specify both 'polygon' and "
                             "'bounds' options.");
        if (!m_bounds.empty())
            m_wkt = m_bounds.toWKT();
    }
    else
    {
        int argc = static_cast<int>(!m_filespec.empty()) +
                   static_cast<int>(!m_listfile.empty()) +
                   static_cast<int>(m_usestdin);
        if (argc > 1)
            throw pdal_error("Can't specify more than one of --glob, "
                             "--filelist or --stdin.");
        if (!argc)
            throw pdal_error("Must specify either --glob, --filelist or"
                             " --stdin.");
        if (m_prefix.size() && m_absPath)
            throw pdal_error("Can't specify both --write_absolute_path and "
                             "--path_prefix options.");
        if (args.set("a_srs"))
            m_overrideASrs = true;
    }
}

int TIndexKernel::execute()
{
    StringList args;
    args.push_back(m_subcommand);
    if (m_subcommand == "create")
    {
        addOption(args, "--tindex", m_idxFilename);
        addOption(args, "--glob", m_filespec);
        addOption(args, "--filelist", m_listfile);
        addFlag(args, "--stdin", m_usestdin);
        addBoolOption(args, "--fast_boundary", m_fastBoundary);
        addOption(args, "--lyr_name", m_layerName);
        addOption(args, "--tindex_name", m_tileIndexColumnName);
        addOption(args, "--ogrdriver", m_driverName);
        for (const std::string& option : m_lcOptions)
            addOption(args, "--lco", option);
        addOption(args, "--t_srs", m_tgtSrsString);
        if (m_overrideASrs)
            addOption(args, "--a_srs", m_assignSrsString);
        addBoolOption(args, "--write_absolute_path", m_absPath);
        addOption(args, "--path_prefix", m_prefix);
        addScalarOption(args, "--threads", m_threads);
        addBoolOption(args, "--skip_different_srs", m_skipMultiSrs);
        addScalarOption(args, "--simplify", m_doSmooth);
        addScalarOption(args, "--threshold", m_density);
        if (m_edgeLength != 0.0)
            addScalarOption(args, "--resolution", m_edgeLength);
        addScalarOption(args, "--sample_size", m_sampleSize);
        addOption(args, "--where", m_boundaryExpr);
    }
    else
    {
        addOption(args, "--tindex", m_idxFilename);
        addOption(args, "--filespec", m_filespec);
        addOption(args, "--lyr_name", m_layerName);
        addOption(args, "--tindex_name", m_tileIndexColumnName);
        addOption(args, "--ogrdriver", m_driverName);
        addOption(args, "--polygon", m_wkt);
        addOption(args, "--t_srs", m_tgtSrsString);
    }

    std::vector<const char*> argv;
    argv.reserve(args.size());
    for (const std::string& arg : args)
        argv.push_back(arg.c_str());

    return pdal_rust_kernel_run("tindex", static_cast<int>(argv.size()),
                                argv.data());
}

} // namespace pdal
