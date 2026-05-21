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
 *
 ****************************************************************************/

#include "StatsFilter.hpp"
#include <pdal/private/RustViewConverter.hpp>
#include <cmath>
#include <pdal/Options.hpp>
#include <pdal/PDALUtils.hpp>
#include <pdal/Polygon.hpp>
#include <pdal/util/ProgramArgs.hpp>
#include <pdal_capi.h>
#include <unordered_map>

// Redundant extern C removed because pdal_capi.h is included.

namespace pdal
{

static StaticPluginInfo const s_info{
    "filters.stats",
    "Compute statistics about each dimension (mean, min, max, etc.)",
    "https://pdal.org/stages/filters.stats.html"};

CREATE_STATIC_STAGE(StatsFilter, s_info)

std::string StatsFilter::getName() const
{
    return s_info.name;
}

namespace stats
{

void Summary::extractMetadata(MetadataNode& m)
{
    uint32_t cnt = static_cast<uint32_t>(count());
    m.add("count", cnt, "count");
    m.add("minimum", minimum(), "minimum");
    m.add("maximum", maximum(), "maximum");
    m.add("average", average(), "average");

    double std = sampleStddev();
    if (!std::isinf(std) && !std::isnan(std))
        m.add("stddev", std, "standard deviation");

    double v = sampleVariance();
    if (!std::isinf(v) && !std::isnan(v))
        m.add("variance", v, "variance");
    m.add("name", m_name, "name");

    if (m_advanced)
    {
        double k = sampleExcessKurtosis();
        if (!std::isinf(k) && !std::isnan(k))
            m.add("kurtosis", k, "kurtosis");

        double sk = sampleSkewness();
        if (!std::isinf(sk) && !std::isnan(sk))
            m.add("skewness", sampleSkewness(), "skewness");
    }

    if (m_enumerate == Enumerate)
    {
        for (auto& v : m_values)
            m.addList("values", v.first);
    }
    else if (m_enumerate == Global)
    {
        m.add("median", m_median);
        m.add("mad", m_mad);
    }
    else if (m_enumerate == Count)
    {
        MetadataNode bins = m.add("bins");
        for (auto& v : m_values)
        {
            bins.add(std::to_string(v.first), uint64_t(v.second));
            m.addList("counts",
                      std::to_string(v.first) + "/" + std::to_string(v.second));
        }
    }
}

void Summary::computeGlobalStats()
{
    if (m_data.empty())
        return;

    std::sort(m_data.begin(), m_data.end());
    size_t mid = m_data.size() / 2;
    m_median = m_data[mid];

    std::vector<double> diffs(m_data.size());
    for (size_t i = 0; i < m_data.size(); ++i)
        diffs[i] = std::abs(m_data[i] - m_median);
    std::sort(diffs.begin(), diffs.end());
    m_mad = diffs[mid];
}

bool Summary::merge(const Summary& s)
{
    if (m_name != s.m_name || m_enumerate != s.m_enumerate ||
        m_advanced != s.m_advanced)
        return false;

    if (s.m_cnt == 0)
        return true;

    if (m_cnt == 0)
    {
        m_min = s.m_min;
        m_max = s.m_max;
        m_cnt = s.m_cnt;
        M1 = s.M1;
        M2 = s.M2;
        M3 = s.M3;
        M4 = s.M4;
        m_median = s.m_median;
        m_mad = s.m_mad;
        m_values = s.m_values;
        m_data = s.m_data;
        return true;
    }

    double n1 = m_cnt;
    double n2 = s.m_cnt;
    double n = n1 + n2;

    double delta = s.M1 - M1;
    double delta2 = delta * delta;
    double delta3 = delta2 * delta;
    double delta4 = delta3 * delta;

    m_cnt = n;
    m_min = std::min(m_min, s.m_min);
    m_max = std::max(m_max, s.m_max);

    M1 = (n1 * M1 + n2 * s.M1) / n;

    if (m_advanced)
    {
        double new_M4 =
            M4 + s.M4 +
            delta4 * n1 * n2 * (n1 * n1 - n1 * n2 + n2 * n2) / (n * n * n) +
            6.0 * delta2 * (n1 * n1 * s.M2 + n2 * n2 * M2) / (n * n) +
            4.0 * delta * (n1 * s.M3 - n2 * M3) / n;
        double new_M3 = M3 + s.M3 + delta3 * n1 * n2 * (n1 - n2) / (n * n) +
                        3.0 * delta * (n1 * s.M2 - n2 * M2) / n;
        M4 = new_M4;
        M3 = new_M3;
    }

    M2 = M2 + s.M2 + delta2 * n1 * n2 / n;

    if (m_enumerate != NoEnum)
    {
        for (auto const& pair : s.m_values)
            m_values[pair.first] += pair.second;
    }

    if (m_enumerate == Global)
    {
        m_data.insert(m_data.end(), s.m_data.begin(), s.m_data.end());
        computeGlobalStats();
    }

    return true;
}

} // namespace stats

using namespace stats;

bool StatsFilter::processOne(PointRef& point)
{
    for (auto p = m_stats.begin(); p != m_stats.end(); ++p)
    {
        Dimension::Id d = p->first;
        Summary& c = p->second;
        c.insert(point.getFieldAs<double>(d));
    }
    return true;
}

void StatsFilter::filter(PointView& view)
{
    if (view.empty())
        return;

    std::vector<std::string> dim_names;
    dim_names.reserve(m_stats.size());
    std::vector<const char*> dims_ptrs;
    dims_ptrs.reserve(m_stats.size());
    for (auto const& pair : m_stats)
    {
        dim_names.push_back(pair.second.m_name);
        dims_ptrs.push_back(dim_names.back().c_str());
    }

    std::vector<const char*> enums_ptrs;
    for (auto const& s : m_enums)
    {
        enums_ptrs.push_back(s.c_str());
    }

    std::vector<const char*> counts_ptrs;
    for (auto const& s : m_counts)
    {
        counts_ptrs.push_back(s.c_str());
    }

    std::vector<const char*> globals_ptrs;
    for (auto const& s : m_global)
    {
        globals_ptrs.push_back(s.c_str());
    }

    std::vector<pdal_dim_stats_t> out_stats(m_stats.size());

    pdal_point_view_t* rust_in = rust_view_converter::toRust(view);

    pdal_stats_compute(
        rust_in, dims_ptrs.data(), dims_ptrs.size(), m_advanced,
        enums_ptrs.empty() ? nullptr : enums_ptrs.data(), enums_ptrs.size(),
        counts_ptrs.empty() ? nullptr : counts_ptrs.data(), counts_ptrs.size(),
        globals_ptrs.empty() ? nullptr : globals_ptrs.data(),
        globals_ptrs.size(), out_stats.data());

    pdal_point_view_destroy(rust_in);

    size_t idx = 0;
    for (auto& pair : m_stats)
    {
        Summary& s = pair.second;
        const auto& rstats = out_stats[idx];
        s.m_cnt = rstats.count;
        s.m_min = rstats.min;
        s.m_max = rstats.max;

        s.M1 = rstats.m1;
        s.M2 = rstats.m2;
        if (s.m_advanced)
        {
            s.M3 = rstats.m3;
            s.M4 = rstats.m4;
        }

        s.m_median = rstats.median;
        s.m_mad = rstats.mad;

        s.m_values.clear();
        for (uint64_t j = 0; j < rstats.unique_len; ++j)
        {
            s.m_values[rstats.unique_values[j]] = rstats.unique_counts[j];
        }

        idx++;
    }

    pdal_free_stats_arrays(out_stats.data(), out_stats.size());
}

void StatsFilter::done(PointTableRef table)
{
    extractMetadata(table);
}

void StatsFilter::addArgs(ProgramArgs& args)
{
    args.add("dimensions", "Dimensions on which to calculate statistics",
             m_dimNames);
    args.add("enumerate", "Dimensions whose values should be enumerated",
             m_enums);
    args.add("global", "Dimensions to compute global stats (median, mad, mode)",
             m_global);
    args.add("count", "Dimensions whose values should be counted", m_counts);
    args.add("advanced", "Calculate skewness and kurtosis", m_advanced);
    args.add("commonsrs", "Common SRS to use for normalizing bounding boxes",
             m_commonSrs, "EPSG:4326");
}

void StatsFilter::prepared(PointTableRef table)
{
    PointLayoutPtr layout(table.layout());
    std::unordered_map<std::string, Summary::EnumType> dims;

    auto getWarn([this]() -> std::ostream&
                 { return log()->get(LogLevel::Warning); });

    if (m_dimNames.empty())
    {
        for (auto id : layout->dims())
            dims[layout->dimName(id)] = Summary::NoEnum;
    }
    else
    {
        for (auto& s : m_dimNames)
        {
            if (layout->findDim(s) == Dimension::Id::Unknown)
                getWarn() << "Dimension '" << s
                          << "' listed in --dimensions "
                             "option does not exist.  Ignoring."
                          << '\n';
            else
                dims[s] = Summary::NoEnum;
        }
    }

    for (auto& s : m_enums)
    {
        if (dims.find(s) == dims.end())
            getWarn() << "Dimension '" << s
                      << "' listed in --enumerate option "
                         "does not exist.  Ignoring."
                      << '\n';
        else
            dims[s] = Summary::Enumerate;
    }

    for (auto& s : m_counts)
    {
        if (dims.find(s) == dims.end())
            getWarn() << "Dimension '" << s
                      << "' listed in --count option "
                         "does not exist.  Ignoring."
                      << '\n';
        else
            dims[s] = Summary::Count;
    }

    for (auto& s : m_global)
    {
        if (dims.find(s) == dims.end())
            getWarn() << "Dimension '" << s
                      << "' listed in --global option "
                         "does not exist.  Ignoring."
                      << '\n';
        else
            dims[s] = Summary::Global;
    }

    for (auto& dv : dims)
        m_stats.insert(
            std::make_pair(layout->findDim(dv.first),
                           Summary(dv.first, dv.second, m_advanced)));
}

void StatsFilter::extractMetadata(PointTableRef table)
{
    uint32_t position(0);

    bool bNoPoints(true);
    for (auto di = m_stats.begin(); di != m_stats.end(); ++di)
    {
        Summary& s = di->second;

        bNoPoints = (bool)s.count();

        MetadataNode t = m_metadata.addList("statistic");
        t.add("position", position++);
        s.extractMetadata(t);
    }

    auto xs = m_stats.find(Dimension::Id::X);
    auto ys = m_stats.find(Dimension::Id::Y);
    auto zs = m_stats.find(Dimension::Id::Z);
    if (xs != m_stats.end() && ys != m_stats.end() && zs != m_stats.end() &&
        bNoPoints)
    {
        BOX3D box(xs->second.minimum(), ys->second.minimum(),
                  zs->second.minimum(), xs->second.maximum(),
                  ys->second.maximum(), zs->second.maximum());
        pdal::Polygon p(box);

        MetadataNode mbox = Utils::toMetadata(box);
        MetadataNode box_metadata = m_metadata.add("bbox");
        MetadataNode metadata = box_metadata.add("native");

        MetadataNode boundary = metadata.addWithType(
            "boundary", p.json(), "json", "GeoJSON boundary");
        MetadataNode bbox = metadata.add(mbox);
        SpatialReference ref = table.anySpatialReference();
        if (!ref.empty())
        {
            p.setSpatialReference(ref);
            if (p.transform(m_commonSrs))
            {
                BOX3D ddbox = p.bounds();
                MetadataNode epsg_4326_box = Utils::toMetadata(ddbox);
                MetadataNode dddbox = box_metadata.add(m_commonSrs);
                dddbox.add(epsg_4326_box);

                MetadataNode ddboundary = dddbox.addWithType(
                    "boundary", p.json(), "json", "GeoJSON boundary");
            }
        }
    }
}

const Summary& StatsFilter::getStats(Dimension::Id dim) const
{
    for (auto di = m_stats.begin(); di != m_stats.end(); ++di)
    {
        Dimension::Id d = di->first;
        if (d == dim)
            return di->second;
    }
    throw pdal_error("filters.stats: Dimension not found.");
}

} // namespace pdal
