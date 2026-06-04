/******************************************************************************
 * Copyright (c) 2016, Hobu Inc. (info@hobu.co)
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

#include <pdal/pdal_test_main.hpp>

#include <pdal_capi.h>

#include <nlohmann/json.hpp>

namespace
{

using Json = NL::json;

std::string takeString(char* raw)
{
    if (!raw)
        return std::string();
    std::string out(raw);
    pdal_string_free(raw);
    return out;
}

Json arg(const std::string& name, const std::string& kind)
{
    return {{"name", name}, {"kind", kind}};
}

Json arg(const std::string& name, const std::string& kind,
         const Json& defaultValue)
{
    return {{"name", name}, {"kind", kind}, {"default", defaultValue}};
}

Json shortArg(const std::string& name, const std::string& shortName,
              const std::string& kind, const Json& defaultValue = Json())
{
    Json spec = {{"name", name}, {"short", shortName}, {"kind", kind}};
    if (!defaultValue.is_null())
        spec["default"] = defaultValue;
    return spec;
}

Json parse(const Json& specs, const Json& args, bool simple = false)
{
    std::string specText = specs.dump();
    std::string argsText = args.dump();
    char* raw = pdal_program_args_parse_json(specText.c_str(), argsText.c_str(),
                                             simple);
    EXPECT_NE(raw, nullptr) << (pdal_last_error() ? pdal_last_error() : "");
    return Json::parse(takeString(raw));
}

void expectFail(const Json& specs, const Json& args, bool simple = false)
{
    std::string specText = specs.dump();
    std::string argsText = args.dump();
    char* raw = pdal_program_args_parse_json(specText.c_str(), argsText.c_str(),
                                             simple);
    EXPECT_EQ(raw, nullptr);
    if (raw)
        pdal_string_free(raw);
}

Json values(const Json& parsed)
{
    return parsed.at("values");
}

Json baseSpecs()
{
    return {shortArg("foo", "f", "string", "foo"), arg("bar", "int", 23),
            shortArg("baz", "z", "bool")};
}

} // namespace

TEST(ProgramArgsTest, t1)
{
    // Json::array() is required for a single-element spec list: under libc++,
    // copy-list-init `Json x = { jsonValue }` with one element that is already
    // a Json prefers the copy constructor and collapses to that object (a map)
    // instead of building a 1-element array. The C ABI expects an array.
    Json specs = Json::array({shortArg("foo", "f", "string", "foo")});

    expectFail(specs, {"--foo"});
    EXPECT_EQ(values(parse(specs, {"--foo=TestFoo"}))["foo"], "TestFoo");
    EXPECT_EQ(values(parse(specs, {"--foo", "TestBar"}))["foo"], "TestBar");
    expectFail(specs, {"-f"});
    expectFail(specs, {"-f", "-g"});
    EXPECT_EQ(values(parse(specs, {"-f", "Gah"}))["foo"], "Gah");
    EXPECT_EQ(values(parse(specs, {"--foo=-Foo"}))["foo"], "-Foo");
}

TEST(ProgramArgsTest, t2)
{
    Json specs = baseSpecs();

    Json parsed = values(parse(specs, {"--foo", "TEst", "--bar=45", "-z"}));
    EXPECT_EQ(parsed["foo"], "TEst");
    EXPECT_EQ(parsed["bar"], 45);
    EXPECT_EQ(parsed["baz"], true);

    parsed = values(parse(specs, {"-zf", "FooTest", "--bar=55"}));
    EXPECT_EQ(parsed["foo"], "FooTest");
    EXPECT_EQ(parsed["bar"], 55);
    EXPECT_EQ(parsed["baz"], true);

    parsed = values(parse(specs, Json::array()));
    EXPECT_EQ(parsed["foo"], "foo");
    EXPECT_EQ(parsed["bar"], 23);
    EXPECT_EQ(parsed["baz"], false);

    expectFail(specs, {"--zf", "Foo"});
    expectFail(specs, {"-fz", "FooTest"});
    EXPECT_EQ(values(parse(specs, {"--foo=This is a test"}))["foo"],
              "This is a test");

    Json truth = shortArg("truth", "t", "bool", true);
    Json truthSpecs = Json::array({truth});
    EXPECT_EQ(values(parse(truthSpecs, {"--truth"}))["truth"], false);
    EXPECT_EQ(values(parse(truthSpecs, {"--truth=true"}))["truth"], true);
    EXPECT_EQ(values(parse(truthSpecs, {"--truth=false"}))["truth"], false);
    expectFail(truthSpecs, {"--truth=flub"});
}

TEST(ProgramArgsTest, t3)
{
    Json specs = baseSpecs();

    expectFail(specs, {"--foo"});
    expectFail(specs, {"--bar=foo"});
    expectFail(specs, {"--bar"});
    expectFail(specs, {"--foo", "--baz"});
    expectFail(specs, {"--flub"});
    expectFail(specs, {"--baz=flub"});
    expectFail(specs, {"-zq"});
    expectFail(specs, {"-q"});
    expectFail(specs, {"-fz", "foo"});
}

TEST(ProgramArgsTest, t4)
{
    expectFail({{{"name", "foo"}, {"short", "f,q"}, {"kind", "string"}}},
               Json::array());
    expectFail({{{"name", "foo"}, {"short", "flub"}, {"kind", "string"}}},
               Json::array());
    expectFail({{{"name", ""}, {"kind", "string"}}}, Json::array());
}

TEST(ProgramArgsTest, synonym)
{
    Json spec = shortArg("foo", "f", "string", "foo");
    spec["aliases"] = {"bar"};
    // Json::array(): single-element spec list, see t1 for why.
    Json specs = Json::array({spec});

    expectFail(specs, {"--bar"});
    EXPECT_EQ(values(parse(specs, {"--bar=TestFoo"}))["foo"], "TestFoo");
}

TEST(ProgramArgsTest, positional)
{
    Json foo = shortArg("foo", "f", "string", "foo");
    foo["positional"] = true;
    Json bar = arg("bar", "int", 23);
    bar["positional"] = true;
    Json specs = {foo, bar, shortArg("baz", "z", "bool")};

    Json parsed = values(parse(specs, {"--foo", "Foo", "-z", "55"}));
    EXPECT_EQ(parsed["foo"], "Foo");
    EXPECT_EQ(parsed["bar"], 55);
    EXPECT_EQ(parsed["baz"], true);

    parsed = values(parse(specs, {"-z", "Flub", "66"}));
    EXPECT_EQ(parsed["foo"], "Flub");
    EXPECT_EQ(parsed["bar"], 66);
    EXPECT_EQ(parsed["baz"], true);

    parsed = values(parse(specs, {"Flub", "66"}));
    EXPECT_EQ(parsed["foo"], "Flub");
    EXPECT_EQ(parsed["bar"], 66);
    EXPECT_EQ(parsed["baz"], false);
}

TEST(ProgramArgsTest, vector)
{
    Json foo = shortArg("foo", "f", "string", "foo");
    foo["positional"] = true;
    Json bar = arg("bar", "int_vec");
    bar["optional_positional"] = true;
    Json specs = {
        foo,
        bar,
        shortArg("baz", "z", "bool"),
        {{"name", "flub"}, {"kind", "int_vec"}, {"default", {1, 3, 5}}}};

    Json parsed =
        values(parse(specs, {"--bar", "23", "--bar", "45", "Foo", "-z"}));
    EXPECT_EQ(parsed["foo"], "Foo");
    EXPECT_EQ(parsed["bar"], Json({23, 45}));
    EXPECT_EQ(parsed["baz"], true);
    EXPECT_EQ(parsed["flub"], Json({1, 3, 5}));

    parsed = values(parse(specs, {"Foo"}));
    EXPECT_EQ(parsed["bar"], Json::array());
    EXPECT_EQ(parsed["foo"], "Foo");
    EXPECT_EQ(parsed["baz"], false);

    parsed = values(parse(specs, {"Fool", "44", "55", "66"}));
    EXPECT_EQ(parsed["foo"], "Fool");
    EXPECT_EQ(parsed["bar"], Json({44, 55, 66}));
    EXPECT_EQ(parsed["baz"], false);

    parsed = values(parse(
        specs, {"--bar", "23", "--flub", "2", "Foo", "-z", "--flub", "4"}));
    EXPECT_EQ(parsed["foo"], "Foo");
    EXPECT_EQ(parsed["bar"], Json({23}));
    EXPECT_EQ(parsed["baz"], true);
    EXPECT_EQ(parsed["flub"], Json({2, 4}));
}

TEST(ProgramArgsTest, stringvector)
{
    Json foo = shortArg("foo", "f", "string", "foo");
    foo["positional"] = true;
    Json bar = arg("bar", "string_vec");
    bar["optional_positional"] = true;
    Json specs = {foo, bar, shortArg("baz", "z", "bool")};

    Json parsed =
        values(parse(specs, {"--bar", "a,b,c", "--bar", "d,e,f", "Foo", "-z"}));
    EXPECT_EQ(parsed["foo"], "Foo");
    EXPECT_EQ(parsed["bar"], Json({"a", "b", "c", "d", "e", "f"}));
    EXPECT_EQ(parsed["baz"], true);

    parsed = values(parse(specs, {"Foo"}));
    EXPECT_EQ(parsed["bar"], Json::array());
    EXPECT_EQ(parsed["foo"], "Foo");
    EXPECT_EQ(parsed["baz"], false);

    parsed = values(parse(specs, {"Fool", "44", "55", "66"}));
    EXPECT_EQ(parsed["foo"], "Fool");
    EXPECT_EQ(parsed["bar"], Json({"44", "55", "66"}));
    EXPECT_EQ(parsed["baz"], false);
}

TEST(ProgramArgsTest, regexvector)
{
    Json parsed =
        values(parse(Json::array({arg("foo", "regex_vec")}),
                     {"--foo", "Yoyoyo\\w{0,10}", "--foo", "Bar|Baz"}));
    EXPECT_EQ(parsed["foo"], Json({"Yoyoyo\\w{0,10}", "Bar|Baz"}));
}

TEST(ProgramArgsTest, vectorfail)
{
    Json bar = arg("bar", "int_vec");
    bar["optional_positional"] = true;
    Json foo = shortArg("foo", "f", "string", "foo");
    foo["positional"] = true;
    expectFail({bar, foo, shortArg("baz", "z", "bool")}, Json::array());
}

TEST(ProgramArgsTest, parseSimple)
{
    Json foo = shortArg("foo", "f", "string", "foo");
    foo["positional"] = true;
    Json vec = arg("vec", "string_vec");
    vec["positional"] = true;
    Json specs = {foo, vec, arg("bar", "int", 23),
                  shortArg("baz", "z", "bool")};

    Json parsed =
        values(parse(specs, {"--foo", "TEst", "--bar=45", "-z"}, true));
    EXPECT_EQ(parsed["foo"], "TEst");
    EXPECT_EQ(parsed["bar"], 45);
    EXPECT_EQ(parsed["baz"], true);

    parsed = values(parse(specs, {"-zf", "FooTest", "--bar=55"}, true));
    EXPECT_EQ(parsed["foo"], "FooTest");
    EXPECT_EQ(parsed["bar"], 55);
    EXPECT_EQ(parsed["baz"], true);

    parsed = values(parse(specs, Json::array(), true));
    EXPECT_EQ(parsed["foo"], "foo");
    EXPECT_EQ(parsed["bar"], 23);
    EXPECT_EQ(parsed["baz"], false);

    Json result =
        parse(specs,
              {"--bar", "55", "Foo", "Barf", "--holy=Holy", "--cow=Moo", "Vec"},
              true);
    parsed = values(result);
    EXPECT_EQ(parsed["foo"], "Foo");
    EXPECT_EQ(parsed["bar"], 55);
    EXPECT_EQ(parsed["baz"], false);
    EXPECT_EQ(parsed["vec"], Json({"Barf", "Vec"}));
    EXPECT_EQ(result["remaining"], Json({"--holy=Holy", "--cow=Moo"}));

    expectFail(specs, {"--bar", "55", "Foo", "Barf"});
}

TEST(ProgramArgsTest, json)
{
    Json specs = {shortArg("json", "j", "json"),
                  shortArg("string", "s", "string")};
    Json parsed = values(parse(specs, Json::array()));
    EXPECT_TRUE(parsed["json"].is_null());

    const std::string object = "{ \"key\": 42 }";
    parsed = values(parse(specs, {"--json", object, "--string", object}));
    EXPECT_EQ(parsed["json"], Json({{"key", 42}}));
    EXPECT_EQ(Json::parse(parsed["string"].get<std::string>()),
              Json({{"key", 42}}));

    parsed = values(parse(specs, {"--json", "[1,2,3]"}));
    EXPECT_EQ(parsed["json"], Json({1, 2, 3}));

    parsed = values(parse(specs, {"--json", "2"}));
    EXPECT_EQ(parsed["json"], 2);

    parsed = values(parse(specs, {"--json", "3.14"}));
    EXPECT_EQ(parsed["json"], 3.14);
}

TEST(ProgramArgsTest, invalidJson)
{
    Json specs = {shortArg("json", "j", "json"),
                  shortArg("string", "s", "string")};

    Json parsed = values(parse(specs, {"--string", "{ invalid JSON here }"}));
    auto parseJson = [](const std::string& value)
    {
        Json parsed = Json::parse(value);
        (void)parsed;
    };
    EXPECT_THROW(parseJson(parsed["string"].get<std::string>()),
                 Json::parse_error);

    expectFail(specs, {"--json", "{ invalid JSON here }"});
    expectFail(specs, {"--json"});
}

TEST(ProgramArgsTest, doubleValues)
{
    Json specs = Json::array({arg("value", "double")});

    Json parsed = values(parse(specs, {"--value=1.23456789012345"}));
    EXPECT_DOUBLE_EQ(parsed["value"].get<double>(), 1.23456789012345);

    parsed = values(parse(specs, {"--value=NaN"}));
    EXPECT_EQ(parsed["value"], "NaN");

    expectFail(specs, {"--value=not-a-number"});
}

TEST(ProgramArgsTest, issue_2155)
{
    // Json::array(): single-element spec list, see t1 for why.
    Json specs = Json::array({arg("foo", "int_vec")});

    Json parsed = values(parse(specs, {"--foo=-1", "--foo", "2", "--foo=-3"}));
    EXPECT_EQ(parsed["foo"], Json({-1, 2, -3}));

    expectFail(specs, {"--foo=-1", "--foo"});
}
