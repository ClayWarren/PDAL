/******************************************************************************
 * Copyright (c) 2016, Hobu Inc., info@hobu.co
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
 *     * Neither the name of Hobu, Inc. nor the
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

#include <pdal/PDALUtils.hpp>
#include <pdal/util/Algorithm.hpp>

#include "../filters/StatsFilter.hpp"
#include "TextReader.hpp"

namespace pdal
{

static StaticPluginInfo const s_info{
    "readers.text",
    "Text Reader",
    "https://pdal.org/stages/readers.text.html",
    {"txt", "csv"}};

CREATE_STATIC_STAGE(TextReader, s_info)

namespace
{

void addOption(pdal_options_t* options, const std::string& key,
               const std::string& value)
{
    pdal_options_add_str(options, key.c_str(), value.c_str());
}

void addOption(pdal_options_t* options, const std::string& key, uint64_t value)
{
    pdal_options_add_u64(options, key.c_str(), value);
}

void throwLastRustError(const std::string& fallback)
{
    const char* message = pdal_last_error();
    if (message && message[0])
        throw pdal_error(message);
    throw pdal_error(fallback);
}

} // namespace

TextReader::~TextReader()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

std::string TextReader::getName() const
{
    return s_info.name;
}

// NOTE: - Forces reading of the entire file.
QuickInfo TextReader::inspect()
{
    QuickInfo qi;
    FixedPointTable t(100);

    StatsFilter f;
    f.setInput(*this);

    f.prepare(t);
    PointLayoutPtr layout = t.layout();
    for (Dimension::Id id : layout->dims())
        qi.m_dimNames.push_back(layout->dimName(id));
    f.execute(t);

    try
    {
        const stats::Summary& xSummary = f.getStats(Dimension::Id::X);
        qi.m_pointCount = xSummary.count();
        qi.m_bounds.minx = xSummary.minimum();
        qi.m_bounds.maxx = xSummary.maximum();
        const stats::Summary& ySummary = f.getStats(Dimension::Id::Y);
        qi.m_bounds.miny = ySummary.minimum();
        qi.m_bounds.maxy = ySummary.maximum();
        const stats::Summary& zSummary = f.getStats(Dimension::Id::Z);
        qi.m_bounds.minz = zSummary.minimum();
        qi.m_bounds.maxz = zSummary.maximum();
        qi.m_valid = true;
    }
    catch (pdal_error&)
    {
    }
    return qi;
}

// Make sure we have a header line.
void TextReader::checkHeader(const std::string& header)
{
    auto it = std::find_if(header.begin(), header.end(),
                           [](char c) { return std::isalpha(c); });

    if (it == header.end())
    {
        fprintf(stderr, "DEBUG: checkHeader emitting warning\n");
        log()->get(LogLevel::Warning)
            << getName() << ": file '" << m_filename
            << "' doesn't appear to contain a header line." << '\n';
    }
}

void TextReader::initialize(PointTableRef table)
{
    m_line = 0;
    m_rustIndex = 0;
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }

    warnIfHeaderMissing();

    pdal_options_t* options = pdal_options_create();
    addOption(options, "filename", m_filename);
    if (m_separatorArg->set())
        addOption(options, "separator", std::string(1, m_separator));
    if (!m_header.empty())
        addOption(options, "header", m_header);
    addOption(options, "skip", static_cast<uint64_t>(m_skip));

    pdal_reader_t* reader = pdal_reader_create_text(options);
    if (!reader)
    {
        pdal_options_destroy(options);
        throwLastRustError("Failed to create Rust text reader.");
    }

    m_rustView = pdal_reader_read_first(reader);
    pdal_reader_destroy(reader);
    pdal_options_destroy(options);
    if (!m_rustView)
        throwLastRustError("Rust text reader failed.");
}

void TextReader::warnIfHeaderMissing()
{
    fprintf(stderr, "DEBUG: warnIfHeaderMissing called, m_header='%s'\n", m_header.c_str());
    if (!m_header.empty())
        return;

    m_istream = Utils::openFile(m_filename, false);
    if (!m_istream)
        throwError("Unable to open text file '" + m_filename + "'.");

    std::string line;
    for (size_t i = 0; i < m_skip && std::getline(*m_istream, line); ++i)
        ;

    if (std::getline(*m_istream, line))
        checkHeader(line);

    Utils::closeFile(m_istream);
    m_istream = nullptr;
}

void TextReader::addArgs(ProgramArgs& args)
{
    m_separatorArg =
        &(args.add("separator",
                   "Separator character that "
                   "overrides special character found in header line",
                   m_separator, ' '));
    args.add("header", "Use this string as the header line.", m_header);
    args.add("skip",
             "Skip this number of lines before attempting to "
             "read the header.",
             m_skip);
}

void TextReader::addDimensions(PointLayoutPtr layout)
{
    m_dims.clear();
    uint64_t dimCount = pdal_point_view_dim_count(m_rustView);
    for (uint64_t idx = 0; idx < dimCount; ++idx)
    {
        char* rawName = pdal_point_view_dim_name(m_rustView, idx);
        if (!rawName)
            continue;
        std::string name(rawName);
        pdal_string_free(rawName);

        Dimension::Id id =
            layout->registerOrAssignDim(name, Dimension::Type::Double);
        if (Utils::contains(m_dims, id) && id != Dimension::Id::Unknown)
            throwError("Duplicate dimension '" + name +
                       "' detected in input file '" + m_filename + "'.");
        m_dims.push_back(id);
    }
}

void TextReader::ready(PointTableRef table)
{
    m_rustIndex = 0;
}

point_count_t TextReader::read(PointViewPtr view, point_count_t numPts)
{
    point_count_t cnt = 0;
    while (cnt < numPts && m_rustIndex < pdal_point_view_length(m_rustView))
    {
        PointId outIdx = view->size();
        view->point(outIdx);
        for (Dimension::Id dim : m_dims)
        {
            view->setField(
                dim, outIdx,
                pdal_point_view_get_f64(m_rustView, m_rustIndex,
                                        view->layout()->dimName(dim).c_str()));
        }
        cnt++;
        m_rustIndex++;
    }
    return cnt;
}

bool TextReader::processOne(PointRef& point)
{
    if (m_rustIndex >= pdal_point_view_length(m_rustView))
        return false;

    for (Dimension::Id dim : m_dims)
    {
        point.setField(dim,
                       pdal_point_view_get_f64(m_rustView, m_rustIndex,
                                               Dimension::name(dim).c_str()));
    }
    m_rustIndex++;
    return true;
}

void TextReader::done(PointTableRef table)
{
    if (m_istream)
    {
        Utils::closeFile(m_istream);
        m_istream = nullptr;
    }
}

} // namespace pdal
