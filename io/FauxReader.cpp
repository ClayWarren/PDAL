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

#include "FauxReader.hpp"

#include <pdal/Options.hpp>
#include <pdal/PointView.hpp>
#include <pdal/util/ProgramArgs.hpp>

#include <ctime>

namespace pdal
{

static StaticPluginInfo const s_info{
    "readers.faux", "Faux Reader", "https://pdal.org/stages/readers.faux.html"};

CREATE_STATIC_STAGE(FauxReader, s_info)

namespace
{

void addOption(pdal_options_t* options, const std::string& key,
               const std::string& value)
{
    pdal_options_add_str(options, key.c_str(), value.c_str());
}

void addOption(pdal_options_t* options, const std::string& key, double value)
{
    pdal_options_add_f64(options, key.c_str(), value);
}

void addOption(pdal_options_t* options, const std::string& key, uint64_t value)
{
    pdal_options_add_u64(options, key.c_str(), value);
}

void throwLastRustError()
{
    const char* message = pdal_last_error();
    if (message && message[0])
        throw pdal_error(message);
    throw pdal_error("Rust FauxReader failed.");
}

std::string modeName(Mode mode)
{
    switch (mode)
    {
    case Mode::Constant:
        return "constant";
    case Mode::Ramp:
        return "ramp";
    case Mode::Uniform:
        return "uniform";
    case Mode::Normal:
        return "normal";
    case Mode::Grid:
        return "grid";
    case Mode::Invalid:
        return "invalid";
    }
    return "ramp";
}

} // namespace

FauxReader::~FauxReader()
{
    if (m_rustView)
        pdal_point_view_destroy(m_rustView);
}

std::string FauxReader::getName() const
{
    return s_info.name;
}

void FauxReader::addArgs(ProgramArgs& args)
{
    args.add("bounds", "X/Y/Z limits", m_bounds, BOX3D(0, 0, 0, 1, 1, 1));
    args.add("mean_x", "X mean", m_mean_x);
    args.add("mean_y", "Y mean", m_mean_y);
    args.add("mean_z", "Z mean", m_mean_z);
    args.add("stdev_x", "X standard deviation", m_stdev_x, 1.0);
    args.add("stdev_y", "Y standard deviation", m_stdev_y, 1.0);
    args.add("stdev_z", "Z standard deviation", m_stdev_z, 1.0);
    args.add("mode", "Point creation mode", m_mode);
    args.add("number_of_returns", "Max number of returns", m_numReturns);
    m_seedArg = &args.add("seed", "Random generator seed", m_seed);
}

void FauxReader::prepared(PointTableRef table)
{
    if (!m_countArg->set() && m_mode != Mode::Grid)
        throwError("Argument 'count' needs a value and none was provided.");
    if (m_numReturns > 10)
        throwError("Option 'number_of_returns' must be in the range [0,10].");

    if (!(m_mode == Mode::Normal || m_mode == Mode::Uniform))
    {
        pdal_options_t* options = pdal_options_create();
        addOption(options, "mode", modeName(m_mode));
        if (m_seedArg->set())
            addOption(options, "seed", (uint64_t)m_seed);
        pdal_reader_t* reader = pdal_reader_create_faux(options);
        pdal_options_destroy(options);
        if (!reader)
            throwError(pdal_last_error());
        pdal_reader_destroy(reader);
    }
}

void FauxReader::initialize()
{
    if (m_mode == Mode::Uniform || m_mode == Mode::Normal)
    {
        if (!m_seedArg->set())
            m_seed = (uint32_t)std::time(nullptr);
    }

    if (usesRustReader())
    {
        createRustView();
        return;
    }

    if (m_mode == Mode::Uniform || m_mode == Mode::Normal)
        m_generator.seed(m_seed);
    if (m_mode == Mode::Grid)
    {
        m_bounds.minx = ceil(m_bounds.minx);
        m_bounds.maxx = ceil(m_bounds.maxx);
        m_bounds.miny = ceil(m_bounds.miny);
        m_bounds.maxy = ceil(m_bounds.maxy);
        m_bounds.minz = ceil(m_bounds.minz);
        m_bounds.maxz = ceil(m_bounds.maxz);
        // Here delX/Y/Z represent the number of points in each direction.
        double count = 1.0;
        if (m_bounds.maxx <= m_bounds.minx)
            m_delX = 0;
        else
        {
            m_delX = m_bounds.maxx - m_bounds.minx;
            count *= m_delX;
        }
        if (m_bounds.maxy <= m_bounds.miny)
            m_delY = 0;
        else
        {
            m_delY = m_bounds.maxy - m_bounds.miny;
            count *= m_delY;
        }
        if (m_bounds.maxz <= m_bounds.minz)
            m_delZ = 0;
        else
        {
            m_delZ = m_bounds.maxz - m_bounds.minz;
            count *= m_delZ;
        }
        if (!m_delX && !m_delY && !m_delZ)
            count = 0;
        if (!Utils::numericCast(count, m_count))
            throwError("Requested range generates more points than supported.");
    }
    else if (m_mode == Mode::Normal)
    {
        // using nd = std::normal_distribution<double>;

        m_normalX.reset(new nd(m_mean_x, m_stdev_x));
        m_normalY.reset(new nd(m_mean_y, m_stdev_y));
        m_normalZ.reset(new nd(m_mean_z, m_stdev_z));
    }
    else if (m_mode == Mode::Uniform)
    {
        // using urd = std::uniform_real_distribution<double>;

        m_uniformX.reset(new urd(m_bounds.minx, m_bounds.maxx));
        m_uniformY.reset(new urd(m_bounds.miny, m_bounds.maxy));
        m_uniformZ.reset(new urd(m_bounds.minz, m_bounds.maxz));
    }
    else if (m_mode == Mode::Invalid)
    {
        // using urd = std::uniform_real_distribution<double>;

        m_uniformX.reset(new urd(m_bounds.minx, m_bounds.maxx));
        m_uniformY.reset(new urd(m_bounds.miny, m_bounds.maxy));
        m_uniformZ.reset(new urd(m_bounds.minz, m_bounds.maxz));
    }
    else
    {
        if (m_count > 1)
        {
            m_delX = (m_bounds.maxx - m_bounds.minx) / (m_count - 1);
            m_delY = (m_bounds.maxy - m_bounds.miny) / (m_count - 1);
            m_delZ = (m_bounds.maxz - m_bounds.minz) / (m_count - 1);
        }
        else
        {
            m_delX = 0;
            m_delY = 0;
            m_delZ = 0;
        }
    }
}

void FauxReader::addDimensions(PointLayoutPtr layout)
{
    Dimension::IdList ids = {Dimension::Id::X, Dimension::Id::Y,
                             Dimension::Id::Z, Dimension::Id::OffsetTime};

    layout->registerDims(ids);
    if (m_numReturns > 0)
    {
        layout->registerDim(Dimension::Id::ReturnNumber);
        layout->registerDim(Dimension::Id::NumberOfReturns);
    }
}

void FauxReader::ready(PointTableRef /*table*/)
{
    m_returnNum = 1;
    m_time = 0;
    m_seed = (uint32_t)std::time(nullptr);
    m_index = 0;
}

#pragma warning(push)
#pragma warning(disable : 4244)
bool FauxReader::processOne(PointRef& point)
{
    if (m_rustView)
    {
        if (m_index >= pdal_point_view_length(m_rustView))
            return false;
        copyRustPoint(point);
        ++m_index;
        return true;
    }

    double x(0);
    double y(0);
    double z(0);

    if (m_index >= m_count)
        return false;

    switch (m_mode)
    {
    case Mode::Constant:
        x = m_bounds.minx;
        y = m_bounds.miny;
        z = m_bounds.minz;
        break;
    case Mode::Ramp:
        x = m_bounds.minx + m_delX * m_index;
        y = m_bounds.miny + m_delY * m_index;
        z = m_bounds.minz + m_delZ * m_index;
        break;
    case Mode::Normal:
        x = (*m_normalX)(m_generator);
        y = (*m_normalY)(m_generator);
        z = (*m_normalZ)(m_generator);
        break;
    case Mode::Uniform:
        x = (*m_uniformX)(m_generator);
        y = (*m_uniformY)(m_generator);
        z = (*m_uniformZ)(m_generator);
        break;
    case Mode::Invalid:
        x = (*m_uniformX)(m_generator);
        y = (*m_uniformY)(m_generator);
        z = (*m_uniformZ)(m_generator);
        break;
    case Mode::Grid:
    {
        if (m_delX)
            x = m_index % (point_count_t)m_delX;

        if (m_delY)
        {
            if (m_delX)
                y = (m_index / (point_count_t)m_delX) % (point_count_t)m_delY;
            else
                y = m_index % (point_count_t)m_delY;
        }

        if (m_delZ)
        {
            if (m_delX && m_delY)
                z = m_index / (point_count_t)(m_delX * m_delY);
            else if (m_delX)
                z = m_index / (point_count_t)m_delX;
            else if (m_delY)
                z = m_index / (point_count_t)m_delY;
        }
        break;
    }
    }

    point.setField(Dimension::Id::X, x);
    point.setField(Dimension::Id::Y, y);
    point.setField(Dimension::Id::Z, z);

    if (m_mode == Mode::Invalid)
        point.setField(Dimension::Id::OffsetTime,
                       std::numeric_limits<double>::quiet_NaN());
    else
        point.setField(Dimension::Id::OffsetTime, m_time++);
    if (m_numReturns > 0)
    {
        point.setField(Dimension::Id::ReturnNumber, m_returnNum);
        point.setField(Dimension::Id::NumberOfReturns, m_numReturns);
        m_returnNum = (m_returnNum % m_numReturns) + 1;
    }
    m_index++;
    return true;
}
#pragma warning(pop)

bool FauxReader::usesRustReader() const
{
    return m_mode == Mode::Constant || m_mode == Mode::Ramp ||
           m_mode == Mode::Uniform || m_mode == Mode::Normal ||
           m_mode == Mode::Grid || m_mode == Mode::Invalid;
}

void FauxReader::createRustView()
{
    if (m_rustView)
    {
        pdal_point_view_destroy(m_rustView);
        m_rustView = nullptr;
    }

    pdal_options_t* options = pdal_options_create();
    addOption(options, "mode", modeName(m_mode));
    addOption(options, "count", (uint64_t)m_count);
    addOption(options, "minx", m_bounds.minx);
    addOption(options, "maxx", m_bounds.maxx);
    addOption(options, "miny", m_bounds.miny);
    addOption(options, "maxy", m_bounds.maxy);
    addOption(options, "minz", m_bounds.minz);
    addOption(options, "maxz", m_bounds.maxz);
    addOption(options, "number_of_returns", (uint64_t)m_numReturns);
    if (m_mode == Mode::Uniform || m_mode == Mode::Normal)
        addOption(options, "seed", (uint64_t)m_seed);
    if (m_mode == Mode::Normal)
    {
        addOption(options, "mean_x", m_mean_x);
        addOption(options, "mean_y", m_mean_y);
        addOption(options, "mean_z", m_mean_z);
        addOption(options, "stdev_x", m_stdev_x);
        addOption(options, "stdev_y", m_stdev_y);
        addOption(options, "stdev_z", m_stdev_z);
    }

    pdal_reader_t* reader = pdal_reader_create_faux(options);
    if (!reader)
    {
        pdal_options_destroy(options);
        throwLastRustError();
    }

    m_rustView = pdal_reader_read_first(reader);
    pdal_reader_destroy(reader);
    pdal_options_destroy(options);
    if (!m_rustView)
        throwLastRustError();
    m_count = pdal_point_view_length(m_rustView);
}

void FauxReader::copyRustPoint(PointRef& point)
{
    point.setField(Dimension::Id::X,
                   pdal_point_view_get_f64(m_rustView, m_index, "X"));
    point.setField(Dimension::Id::Y,
                   pdal_point_view_get_f64(m_rustView, m_index, "Y"));
    point.setField(Dimension::Id::Z,
                   pdal_point_view_get_f64(m_rustView, m_index, "Z"));
    point.setField(Dimension::Id::OffsetTime,
                   pdal_point_view_get_f64(m_rustView, m_index, "OffsetTime"));

    if (m_numReturns > 0)
    {
        point.setField(
            Dimension::Id::ReturnNumber,
            pdal_point_view_get_f64(m_rustView, m_index, "ReturnNumber"));
        point.setField(
            Dimension::Id::NumberOfReturns,
            pdal_point_view_get_f64(m_rustView, m_index, "NumberOfReturns"));
    }
}

point_count_t FauxReader::read(PointViewPtr view, point_count_t count)
{
    for (PointId idx = 0; idx < count; ++idx)
    {
        PointRef point = view->point(idx);
        if (!processOne(point))
            break;
        if (m_cb)
            m_cb(*view, idx);
    }
    return count;
}

} // namespace pdal
