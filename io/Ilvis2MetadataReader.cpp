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

#include "Ilvis2MetadataReader.hpp"

#include <rust/pdal-capi/include/pdal_capi.h>

namespace pdal
{

namespace
{

std::string takeString(char* value)
{
    std::string output(value ? value : "");
    pdal_string_free(value);
    return output;
}

MetadataNode addMetadataChild(MetadataNode& parent,
                              const pdal_metadata_node_t* rustNode)
{
    std::string name = takeString(pdal_metadata_node_name(rustNode));
    std::string description =
        takeString(pdal_metadata_node_description(rustNode));
    uint8_t valueKind = pdal_metadata_node_value_kind(rustNode);

    switch (valueKind)
    {
    case 1:
        return parent.add(name, pdal_metadata_node_value_i64(rustNode),
                          description);
    case 2:
        return parent.add(name, pdal_metadata_node_value_u64(rustNode),
                          description);
    case 3:
        return parent.add(name, pdal_metadata_node_value_f64(rustNode),
                          description);
    case 4:
        return parent.add(name, pdal_metadata_node_value_bool(rustNode),
                          description);
    case 0:
        return parent.add(name, takeString(pdal_metadata_node_value(rustNode)),
                          description);
    default:
        return parent.add(name);
    }
}

void copyMetadataChildren(const pdal_metadata_node_t* rustNode,
                          MetadataNode& cppNode)
{
    uint64_t childCount = pdal_metadata_node_child_count(rustNode);
    for (uint64_t i = 0; i < childCount; ++i)
    {
        pdal_metadata_node_t* rustChild = pdal_metadata_node_child(rustNode, i);
        MetadataNode cppChild = addMetadataChild(cppNode, rustChild);
        copyMetadataChildren(rustChild, cppChild);
        pdal_metadata_node_destroy(rustChild);
    }
}

void throwLastRustError()
{
    const char* message = pdal_last_error();
    if (message && message[0])
        throw Ilvis2MetadataReader::error(message);
    throw Ilvis2MetadataReader::error("Unable to read ILVIS2 metadata.");
}

} // namespace

void Ilvis2MetadataReader::readMetadataFile(std::string filename,
                                            MetadataNode* metadata)
{
    pdal_metadata_node_t* rustMetadata =
        pdal_ilvis2_metadata_read(filename.c_str());
    if (!rustMetadata)
        throwLastRustError();

    copyMetadataChildren(rustMetadata, *metadata);
    pdal_metadata_node_destroy(rustMetadata);
}

} // namespace pdal
